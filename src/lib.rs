use anyhow;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::mem::size_of;
use std::ops::DerefMut;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Protocol {
    NewClientRequest { address: String },
    NewClientResponse,
}

pub async fn read_protocol<R: AsyncRead + std::marker::Unpin>(
    mut reader: impl DerefMut<Target = R>,
) -> Result<Protocol, anyhow::Error> {
    let mut buf_protocol = [0; 128];
    let n = reader.deref_mut().read(&mut buf_protocol).await?;

    Ok(bincode::deserialize(&buf_protocol[0..n])?)
}

pub async fn write_protocol<W: AsyncWrite + AsyncWriteExt + std::marker::Unpin>(
    mut writer: impl DerefMut<Target = W>,
    protocol: &Protocol,
) -> Result<(), anyhow::Error> {
    Ok(writer
        .deref_mut()
        .write_all(&bincode::serialize(protocol)?)
        .await?)
}

pub struct PortAssigner {
    port_start: u16,
}

impl PortAssigner {
    pub fn new() -> Self {
        Self { port_start: 10000 }
    }
    pub fn next(&mut self) -> u16 {
        let ret = self.port_start;
        self.port_start += 1;
        ret
    }
}
