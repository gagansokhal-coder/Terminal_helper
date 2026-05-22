use std::{
    env,
    path::{Path, PathBuf},
};

use crate::{
    error::{DaemonError, DaemonResult},
    platform::IpcEndpoint,
};

const DEFAULT_QUEUE_CAPACITY: usize = 1024;
const DEFAULT_MAX_RETRIES: u8 = 3;
const DEFAULT_IDLE_MEMORY_TARGET_MB: u64 = 50;
const SOCKET_FILE_NAME: &str = "daemon.sock";
const DATABASE_FILE_NAME: &str = "ggnmem.db";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonConfig {
    pub endpoint: IpcEndpoint,
    pub database_path: PathBuf,
    pub queue_capacity: usize,
    pub max_retries: u8,
    pub idle_memory_target_mb: u64,
}

impl DaemonConfig {
    pub fn load() -> DaemonResult<Self> {
        Ok(Self {
            endpoint: default_endpoint()?,
            database_path: default_database_path()?,
            queue_capacity: parse_env_usize("GGNMEM_QUEUE_CAPACITY", DEFAULT_QUEUE_CAPACITY)?,
            max_retries: parse_env_u8("GGNMEM_QUEUE_MAX_RETRIES", DEFAULT_MAX_RETRIES)?,
            idle_memory_target_mb: parse_env_u64(
                "GGNMEM_IDLE_MEMORY_TARGET_MB",
                DEFAULT_IDLE_MEMORY_TARGET_MB,
            )?,
        })
    }

    #[must_use]
    pub fn new(endpoint: IpcEndpoint, database_path: PathBuf) -> Self {
        Self {
            endpoint,
            database_path,
            queue_capacity: DEFAULT_QUEUE_CAPACITY,
            max_retries: DEFAULT_MAX_RETRIES,
            idle_memory_target_mb: DEFAULT_IDLE_MEMORY_TARGET_MB,
        }
    }

    #[must_use]
    pub fn with_queue_capacity(mut self, queue_capacity: usize) -> Self {
        self.queue_capacity = queue_capacity.max(1);
        self
    }

    #[must_use]
    pub fn with_max_retries(mut self, max_retries: u8) -> Self {
        self.max_retries = max_retries;
        self
    }
}

fn parse_env_usize(name: &str, default: usize) -> DaemonResult<usize> {
    match env::var(name) {
        Ok(value) => value
            .parse::<usize>()
            .map(|parsed| parsed.max(1))
            .map_err(|_| DaemonError::InvalidConfig(format!("{name} must be a positive integer"))),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(DaemonError::InvalidConfig(format!(
            "{name} could not be read: {error}"
        ))),
    }
}

fn parse_env_u8(name: &str, default: u8) -> DaemonResult<u8> {
    match env::var(name) {
        Ok(value) => value
            .parse::<u8>()
            .map_err(|_| DaemonError::InvalidConfig(format!("{name} must fit in u8"))),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(DaemonError::InvalidConfig(format!(
            "{name} could not be read: {error}"
        ))),
    }
}

fn parse_env_u64(name: &str, default: u64) -> DaemonResult<u64> {
    match env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| DaemonError::InvalidConfig(format!("{name} must fit in u64"))),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(DaemonError::InvalidConfig(format!(
            "{name} could not be read: {error}"
        ))),
    }
}

#[cfg(unix)]
fn default_endpoint() -> DaemonResult<IpcEndpoint> {
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or(DaemonError::MissingRuntimeDir)?;
    let socket_path = match env::var_os("GGNMEM_SOCKET_PATH") {
        Some(path) => {
            let path = PathBuf::from(path);
            ensure_under_runtime_dir(&path, &runtime_dir)?;
            path
        }
        None => runtime_dir.join("ggnmem").join(SOCKET_FILE_NAME),
    };

    Ok(IpcEndpoint::Unix(socket_path))
}

#[cfg(windows)]
fn default_endpoint() -> DaemonResult<IpcEndpoint> {
    let pipe_name =
        env::var("GGNMEM_NAMED_PIPE").unwrap_or_else(|_| r"\\.\pipe\ggnmem_ipc".to_owned());
    Ok(IpcEndpoint::NamedPipe(pipe_name))
}

#[cfg(unix)]
fn ensure_under_runtime_dir(path: &Path, runtime_dir: &Path) -> DaemonResult<()> {
    if path.starts_with(runtime_dir) {
        return Ok(());
    }

    Err(DaemonError::InvalidConfig(format!(
        "Linux socket path must live under XDG_RUNTIME_DIR: {}",
        path.display()
    )))
}

fn default_database_path() -> DaemonResult<PathBuf> {
    if let Some(path) = env::var_os("GGNMEM_DATABASE_PATH") {
        return Ok(PathBuf::from(path));
    }

    #[cfg(unix)]
    {
        if let Some(data_home) = env::var_os("XDG_DATA_HOME") {
            return Ok(PathBuf::from(data_home)
                .join("ggnmem")
                .join(DATABASE_FILE_NAME));
        }

        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| DaemonError::InvalidConfig("HOME is not set".to_owned()))?;
        Ok(home
            .join(".local")
            .join("share")
            .join("ggnmem")
            .join(DATABASE_FILE_NAME))
    }

    #[cfg(windows)]
    {
        let app_data = env::var_os("APPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| DaemonError::InvalidConfig("APPDATA is not set".to_owned()))?;
        Ok(app_data.join("ggnmem").join(DATABASE_FILE_NAME))
    }
}
