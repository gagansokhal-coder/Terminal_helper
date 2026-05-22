use std::{path::PathBuf, pin::Pin};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};

use crate::{
    error::{DaemonError, DaemonResult},
    platform::{IpcEndpoint, Platform},
};

pub struct PlatformListener {
    path: PathBuf,
    listener: UnixListener,
}

impl PlatformListener {
    pub async fn bind(endpoint: &IpcEndpoint) -> DaemonResult<Self> {
        let IpcEndpoint::Unix(path) = endpoint else {
            return Err(DaemonError::InvalidConfig(
                "Linux listener requires Unix socket endpoint".to_owned(),
            ));
        };

        if path.exists() {
            std::fs::remove_file(path)?;
        }

        let listener = UnixListener::bind(path)?;
        Ok(Self {
            path: path.clone(),
            listener,
        })
    }

    pub async fn accept(&mut self) -> DaemonResult<PlatformConnection> {
        let (stream, _) = self.listener.accept().await?;
        Ok(PlatformConnection { stream })
    }

    pub async fn shutdown(&mut self) -> DaemonResult<()> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}

pub struct PlatformConnection {
    stream: UnixStream,
}

impl PlatformConnection {
    pub async fn connect(endpoint: &IpcEndpoint) -> DaemonResult<Self> {
        let IpcEndpoint::Unix(path) = endpoint else {
            return Err(DaemonError::InvalidConfig(
                "Linux client requires Unix socket endpoint".to_owned(),
            ));
        };

        Ok(Self {
            stream: UnixStream::connect(path).await?,
        })
    }
}

impl Platform for PlatformConnection {
    fn read_exact<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            self.stream.read_exact(buf).await?;
            Ok(())
        })
    }

    fn write_all<'a>(
        &'a mut self,
        buf: &'a [u8],
    ) -> Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'a>> {
        Box::pin(async move { self.stream.write_all(buf).await })
    }

    fn flush<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'a>> {
        Box::pin(async move { self.stream.flush().await })
    }

    fn shutdown<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'a>> {
        Box::pin(async move { self.stream.shutdown().await })
    }
}
