use anyhow;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::mem::size_of;
use std::ops::{DerefMut, Range, RangeInclusive};
use std::str::FromStr;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum ProtocolSeverToAgent {
    NewClientRequest { address: String },
    NewAgentResponse { address: String },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum ProtocolAgentToSever {
    NewClientResponse,
}

pub async fn read_protocol<R: AsyncRead + std::marker::Unpin, P>(
    mut reader: impl DerefMut<Target = R>,
) -> Result<P, anyhow::Error>
where
    P: for<'de> Deserialize<'de>,
{
    let mut buf_protocol = [0; 128];
    let n = reader.deref_mut().read(&mut buf_protocol).await?;

    Ok(bincode::deserialize(&buf_protocol[0..n])?)
}

pub async fn write_protocol<W: AsyncWrite + AsyncWriteExt + std::marker::Unpin, P: Serialize>(
    mut writer: impl DerefMut<Target = W>,
    protocol: &P,
) -> Result<(), anyhow::Error> {
    Ok(writer
        .deref_mut()
        .write_all(&bincode::serialize(protocol)?)
        .await?)
}

#[derive(Debug, Clone)]
pub struct PortRange {
    range: RangeInclusive<u16>,
}

impl PortRange {
    fn new(range: RangeInclusive<u16>) -> Self {
        Self { range }
    }
}

impl FromStr for PortRange {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ports: Vec<_> = s.split(":").filter_map(|x| x.parse().ok()).collect();

        if ports.len() == 2 {
            Ok(Self {
                range: ports[0]..=ports[1],
            })
        } else {
            Err(format!("parse error: {}", s))
        }
    }
}

pub struct PortAssigner {
    port_range: PortRange,
    current: PortRange,
}

impl PortAssigner {
    pub fn new(port_range: PortRange) -> Self {
        Self {
            current: port_range.clone(),
            port_range: port_range.clone(),
        }
    }
    pub fn next(&mut self) -> u16 {
        if let Some(n) = self.current.range.next() {
            n
        } else {
            self.current = self.port_range.clone();
            self.current.range.next().unwrap()
        }
    }
}
