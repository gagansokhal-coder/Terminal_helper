#![cfg(unix)]

use std::time::Duration;

use ggnmem_daemon::{
    platform::IpcEndpoint,
    protocol::{
        CommandPayload, DaemonRequest, DaemonResponse, DaemonResponseKind, SessionPayload,
        PROTOCOL_VERSION,
    },
    Daemon, DaemonConfig, IpcClient,
};
use ggnmem_db::{hash::content_hash, Database, DatabaseConfig};
use tempfile::TempDir;
use tokio::{sync::oneshot, time};

#[tokio::test]
async fn daemon_ping_health_queue_and_db_ingestion_work() {
    let temp = TempDir::new().expect("temp dir");
    let endpoint = IpcEndpoint::Unix(temp.path().join("runtime").join("daemon.sock"));
    let database_path = temp.path().join("data").join("ggnmem.db");
    let config = DaemonConfig::new(endpoint.clone(), database_path.clone())
        .with_queue_capacity(4)
        .with_max_retries(1);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let daemon = tokio::spawn(async move {
        Daemon::new(config)
            .run_until_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    wait_for_daemon(&endpoint).await;

    let ping = daemon_request(&endpoint, DaemonRequest::ping())
        .await
        .expect("ping response");
    assert_eq!(ping.kind, DaemonResponseKind::Pong);

    let health = daemon_request(&endpoint, DaemonRequest::health())
        .await
        .expect("health response");
    let DaemonResponseKind::Health(status) = health.kind else {
        panic!("expected health response");
    };
    assert_eq!(status.queue_capacity, 4);
    assert!(status.db_connected);

    let session = SessionPayload {
        session_id: "session-1".to_owned(),
        os_context: "linux".to_owned(),
        hostname: "devbox".to_owned(),
        shell: Some("zsh".to_owned()),
        started_at_ms: 1_725_000_000_000,
    };
    let command = CommandPayload {
        command_id: "command-1".to_owned(),
        session_id: session.session_id.clone(),
        command: "git status".to_owned(),
        cwd: "/workspace/ggnmem".to_owned(),
        exit_code: Some(0),
        duration_ms: Some(7),
        started_at_ms: Some(1_725_000_000_010),
        completed_at_ms: 1_725_000_000_017,
    };

    let accepted = daemon_request(
        &endpoint,
        DaemonRequest::IngestCommand {
            version: PROTOCOL_VERSION,
            session: Box::new(session),
            command: Box::new(command),
        },
    )
    .await
    .expect("ingest response");
    assert!(matches!(
        accepted.kind,
        DaemonResponseKind::Accepted { queue_depth: _ }
    ));

    wait_for_command(&database_path, "git status", "/workspace/ggnmem").await;

    shutdown_tx.send(()).expect("shutdown signal");
    time::timeout(Duration::from_secs(5), daemon)
        .await
        .expect("daemon exits")
        .expect("join ok")
        .expect("daemon ok");
}

async fn wait_for_daemon(endpoint: &IpcEndpoint) {
    for _ in 0..100 {
        if daemon_request(endpoint, DaemonRequest::ping())
            .await
            .is_ok()
        {
            return;
        }
        time::sleep(Duration::from_millis(20)).await;
    }

    panic!("daemon did not start");
}

async fn wait_for_command(database_path: &std::path::Path, command: &str, cwd: &str) {
    for _ in 0..100 {
        if let Ok(database) = Database::open(&DatabaseConfig::new(database_path.to_path_buf())) {
            if database
                .get_command_by_hash(&content_hash(command, cwd))
                .expect("command lookup")
                .is_some()
            {
                return;
            }
        }
        time::sleep(Duration::from_millis(20)).await;
    }

    panic!("command was not persisted");
}

async fn daemon_request(
    endpoint: &IpcEndpoint,
    request: DaemonRequest,
) -> ggnmem_daemon::DaemonResult<DaemonResponse> {
    let mut client = IpcClient::connect(endpoint).await?;
    client.request(&request).await
}
