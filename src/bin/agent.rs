use std::net::SocketAddr;
use std::sync::Arc;
use structopt::StructOpt;
use tcp_tunnel_rs::{read_protocol, write_protocol, ProtocolAgentToSever, ProtocolSeverToAgent};
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

const BUFFER_SIZE: usize = 32 * 1024;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short = "c", long = "control_address", parse(try_from_str))]
    control_address: SocketAddr,
    #[structopt(short = "p", long = "target_port")]
    target_port: u16,
}

async fn agent_main(mut control_server_stream: TcpStream, target_port: u16) -> anyhow::Result<()> {
    loop {
        let protocol = { read_protocol(&mut control_server_stream).await.unwrap() };

        match protocol {
            ProtocolSeverToAgent::NewClientRequest { address } => {
                let mut tunnel_server_stream = TcpStream::connect(address.clone()).await.unwrap();
                println!("connect {}", &address);
                let mut target_server_stream =
                    TcpStream::connect(format!("127.0.0.1:{}", target_port))
                        .await
                        .unwrap();
                println!("connect target");
                write_protocol(
                    &mut control_server_stream,
                    &ProtocolAgentToSever::NewClientResponse,
                )
                .await
                .unwrap();
                println!("send request");
                tokio::spawn(async move {
                    let mut buf_tunnel = [0; BUFFER_SIZE];
                    let mut buf_target = [0; BUFFER_SIZE];

                    loop {
                        tokio::select! {
                            n = tunnel_server_stream.read(&mut buf_tunnel) => {
                                let n = match n {
                                    Ok(n) if n == 0 => {
                                        println!("close tunnel server");
                                        return;
                                    },
                                    Ok(n) => n,
                                    Err(e) => {
                                        eprintln!("failed to read from socket; err = {:?}", e);
                                        return;
                                    }
                                };
                                println!("tunnel -> target {}", n);
                                target_server_stream.write_all(&buf_tunnel[0..n]).await.unwrap();
                            }
                            n = target_server_stream.read(&mut buf_target) => {
                                let n = match n {
                                    Ok(n) if n == 0 => {
                                        println!("close target server");
                                        return;
                                    },
                                    Ok(n) => n,
                                    Err(e) => {
                                        eprintln!("failed to read from socket; err = {:?}", e);
                                        return;
                                    }
                                };
                                println!("target -> tunnel {}", n);
                                tunnel_server_stream.write_all(&buf_target[0..n]).await.unwrap();
                            }
                        }
                    }
                });
            }
            ProtocolSeverToAgent::NewAgentResponse { address } => {
                println!("tunnel server address {}", address);
            }
            _ => {
                unreachable!()
            }
        };
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    loop {
        println!("try connect control sever");
        if let Ok(mut control_server_stream) = TcpStream::connect(opt.control_address).await {
            agent_main(control_server_stream, opt.target_port).await?;
        } else {
            println!("fail connect control server.");
            sleep(Duration::from_secs(1)).await;
        }
    }
}
