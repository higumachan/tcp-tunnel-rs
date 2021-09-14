use std::sync::Arc;
use tcp_tunnel_rs::{read_protocol, write_protocol, Protocol};
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;

const BUFFER_SIZE: usize = 32 * 1024;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut control_server_stream =
        Arc::new(RwLock::new(TcpStream::connect("127.0.0.1:7136").await?));

    loop {
        let control_server_stream = control_server_stream.clone();
        let protocol = {
            read_protocol(&mut control_server_stream.write().await)
                .await
                .unwrap()
        };

        match protocol {
            Protocol::NewClientRequest { address } => {
                let mut tunnel_server_stream = TcpStream::connect(address.clone()).await.unwrap();
                println!("connect {}", &address);
                let mut target_server_stream = TcpStream::connect("127.0.0.1:7134").await.unwrap();
                println!("connect target");
                let mut css = control_server_stream.write().await;
                dbg!("get lock");
                write_protocol(&mut css, &Protocol::NewClientResponse)
                    .await
                    .unwrap();
                println!("send request");
                tokio::spawn(async move {
                    let mut buf_tunnel = [0; BUFFER_SIZE];
                    let mut buf_target = [0; BUFFER_SIZE];

                    loop {
                        tokio::select! {
                            n = tunnel_server_stream.read(&mut buf_tunnel) => {
                                let n = n.unwrap();
                                println!("tunnel -> target {}", n);
                                target_server_stream.write_all(&buf_tunnel[0..n]).await.unwrap();
                            }
                            n = target_server_stream.read(&mut buf_target) => {
                                let n = n.unwrap();
                                println!("target -> tunnel {}", n);
                                tunnel_server_stream.write_all(&buf_target[0..n]).await.unwrap();
                            }
                        }
                    }
                });
            }
            _ => {
                unreachable!()
            }
        };
    }
}
