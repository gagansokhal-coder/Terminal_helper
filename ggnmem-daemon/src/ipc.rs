use serde::{de::DeserializeOwned, Serialize};

use crate::{
    error::{DaemonError, DaemonResult},
    platform::{IpcEndpoint, Platform, PlatformListener},
};

const MAX_FRAME_BYTES: usize = 1024 * 1024;

pub struct IpcServer {
    listener: PlatformListener,
}

impl IpcServer {
    pub async fn bind(endpoint: &IpcEndpoint) -> DaemonResult<Self> {
        Ok(Self {
            listener: crate::platform::PlatformListener::bind(endpoint).await?,
        })
    }

    pub async fn accept(&mut self) -> DaemonResult<IpcConnection> {
        Ok(IpcConnection {
            connection: self.listener.accept().await?,
        })
    }

    pub async fn shutdown(&mut self) -> DaemonResult<()> {
        self.listener.shutdown().await
    }
}

pub struct IpcClient<C: Platform = crate::platform::PlatformConnection> {
    connection: C,
}

impl IpcClient {
    pub async fn connect(endpoint: &IpcEndpoint) -> DaemonResult<Self> {
        Ok(Self {
            connection: crate::platform::PlatformConnection::connect(endpoint).await?,
        })
    }

    pub async fn request<Req, Res>(&mut self, request: &Req) -> DaemonResult<Res>
    where
        Req: Serialize,
        Res: DeserializeOwned,
    {
        self.send(request).await?;
        self.receive().await
    }
}

pub struct IpcConnection<C: Platform = crate::platform::PlatformConnection> {
    connection: C,
}

impl<C: Platform> IpcClient<C> {
    pub async fn send<T>(&mut self, value: &T) -> DaemonResult<()>
    where
        T: Serialize,
    {
        send_frame(&mut self.connection, value).await
    }

    pub async fn receive<T>(&mut self) -> DaemonResult<T>
    where
        T: DeserializeOwned,
    {
        receive_frame(&mut self.connection).await
    }

    pub async fn shutdown(&mut self) -> DaemonResult<()> {
        self.connection.shutdown().await?;
        Ok(())
    }
}

impl<C: Platform> IpcConnection<C> {
    pub async fn send<T>(&mut self, value: &T) -> DaemonResult<()>
    where
        T: Serialize,
    {
        send_frame(&mut self.connection, value).await
    }

    pub async fn receive<T>(&mut self) -> DaemonResult<T>
    where
        T: DeserializeOwned,
    {
        receive_frame(&mut self.connection).await
    }

    pub async fn shutdown(&mut self) -> DaemonResult<()> {
        self.connection.shutdown().await?;
        Ok(())
    }
}

async fn send_frame<C, T>(connection: &mut C, value: &T) -> DaemonResult<()>
where
    C: Platform,
    T: Serialize,
{
    let bytes = bincode::serialize(value)?;
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(DaemonError::FrameTooLarge(bytes.len()));
    }

    let len = u32::try_from(bytes.len()).map_err(|_| DaemonError::FrameTooLarge(bytes.len()))?;
    connection.write_all(&len.to_le_bytes()).await?;
    connection.write_all(&bytes).await?;
    connection.flush().await?;
    Ok(())
}

async fn receive_frame<C, T>(connection: &mut C) -> DaemonResult<T>
where
    C: Platform,
    T: DeserializeOwned,
{
    let mut len = [0_u8; 4];
    connection.read_exact(&mut len).await?;
    let len = u32::from_le_bytes(len) as usize;
    if len > MAX_FRAME_BYTES {
        return Err(DaemonError::FrameTooLarge(len));
    }

    let mut bytes = vec![0_u8; len];
    connection.read_exact(&mut bytes).await?;
    Ok(bincode::deserialize(&bytes)?)
}
