use std::{future::Future, path::PathBuf, pin::Pin};

use crate::error::DaemonResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcEndpoint {
    Unix(PathBuf),
    NamedPipe(String),
}

impl IpcEndpoint {
    #[must_use]
    pub fn platform_name(&self) -> &'static str {
        match self {
            Self::Unix(_) => "linux-unix-socket",
            Self::NamedPipe(_) => "windows-named-pipe",
        }
    }

    pub fn prepare_parent_dir(&self) -> DaemonResult<()> {
        match self {
            Self::Unix(path) => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                Ok(())
            }
            Self::NamedPipe(_) => Ok(()),
        }
    }
}

pub trait Platform: Send + Unpin {
    fn read_exact<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + 'a>>;

    fn write_all<'a>(
        &'a mut self,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + 'a>>;

    fn flush<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + 'a>>;

    fn shutdown<'a>(&'a mut self)
        -> Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + 'a>>;
}

#[cfg(unix)]
pub mod linux;
#[cfg(windows)]
pub mod windows;

#[cfg(unix)]
pub use linux::{PlatformConnection, PlatformListener};
#[cfg(windows)]
pub use windows::{PlatformConnection, PlatformListener};
