use std::pin::Pin;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::{ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions},
};

use crate::{
    error::{DaemonError, DaemonResult},
    platform::{IpcEndpoint, Platform},
};

pub struct PlatformListener {
    pipe_name: String,
}

impl PlatformListener {
    pub async fn bind(endpoint: &IpcEndpoint) -> DaemonResult<Self> {
        let IpcEndpoint::NamedPipe(pipe_name) = endpoint else {
            return Err(DaemonError::InvalidConfig(
                "Windows listener requires named pipe endpoint".to_owned(),
            ));
        };

        Ok(Self {
            pipe_name: pipe_name.clone(),
        })
    }

    pub async fn accept(&mut self) -> DaemonResult<PlatformConnection> {
        let server = ServerOptions::new().create(&self.pipe_name)?;
        server.connect().await?;
        Ok(PlatformConnection {
            stream: PipeStream::Server(server),
        })
    }

    pub async fn shutdown(&mut self) -> DaemonResult<()> {
        Ok(())
    }
}

pub struct PlatformConnection {
    stream: PipeStream,
}

impl PlatformConnection {
    pub async fn connect(endpoint: &IpcEndpoint) -> DaemonResult<Self> {
        let IpcEndpoint::NamedPipe(pipe_name) = endpoint else {
            return Err(DaemonError::InvalidConfig(
                "Windows client requires named pipe endpoint".to_owned(),
            ));
        };

        Ok(Self {
            stream: PipeStream::Client(ClientOptions::new().open(pipe_name)?),
        })
    }
}

enum PipeStream {
    Server(NamedPipeServer),
    Client(NamedPipeClient),
}

impl Platform for PlatformConnection {
    fn read_exact<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            match &mut self.stream {
                PipeStream::Server(stream) => {
                    stream.read_exact(buf).await?;
                }
                PipeStream::Client(stream) => {
                    stream.read_exact(buf).await?;
                }
            }
            Ok(())
        })
    }

    fn write_all<'a>(
        &'a mut self,
        buf: &'a [u8],
    ) -> Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            match &mut self.stream {
                PipeStream::Server(stream) => stream.write_all(buf).await,
                PipeStream::Client(stream) => stream.write_all(buf).await,
            }
        })
    }

    fn flush<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            match &mut self.stream {
                PipeStream::Server(stream) => stream.flush().await,
                PipeStream::Client(stream) => stream.flush().await,
            }
        })
    }

    fn shutdown<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            match &mut self.stream {
                PipeStream::Server(stream) => stream.shutdown().await,
                PipeStream::Client(stream) => stream.shutdown().await,
            }
        })
    }
}
