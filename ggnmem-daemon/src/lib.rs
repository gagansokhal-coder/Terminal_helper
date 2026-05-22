pub mod config;
pub mod daemon;
pub mod error;
pub mod health;
pub mod ipc;
pub mod logging;
pub mod platform;
pub mod protocol;
pub mod queue;
pub mod storage;

pub use config::DaemonConfig;
pub use daemon::Daemon;
pub use error::{DaemonError, DaemonResult};
pub use health::{HealthState, HealthStatus};
pub use ipc::{IpcClient, IpcServer};
pub use protocol::{
    CommandPayload, CommandSummary, DaemonRequest, DaemonResponse, DaemonResponseKind,
    ProtocolVersion, SearchResultSummary, SessionPayload, PROTOCOL_VERSION,
};
