#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ggnmem_daemon::daemon::run_loaded_config().await?;
    Ok(())
}
