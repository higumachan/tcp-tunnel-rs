use std::sync::Arc;
use structopt::StructOpt;
use tcp_tunnel_rs::{read_protocol, write_protocol, PortAssigner, Protocol};
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

const BUFFER_SIZE: usize = 32 * 1024;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short = "c", long = "control_address")]
    control_address: String,
    #[structopt(short = "C", long = "client_address")]
    client_address: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let listener_for_agent_control = TcpListener::bind(opt.control_address.clone()).await?;
    let mut port_assigner = Arc::new(RwLock::new(PortAssigner::new()));

    println!("start server");
    loop {
        let port_assigner = port_assigner.clone();
        let mut socket_agent_control = listener_for_agent_control.accept().await?.0;
        println!("connect agent");
        let listener_for_client = TcpListener::bind(opt.client_address.clone()).await?;
        tokio::spawn(async move {
            loop {
                let (mut socket_client, _) = listener_for_client.accept().await.unwrap();
                println!("connect client");

                let new_port = port_assigner.write().await.next();
                let new_address = format!("0.0.0.0:{}", new_port);
                let listener_for_agent = TcpListener::bind(new_address.clone()).await.unwrap();
                println!("bind {}", new_address);
                write_protocol(
                    &mut socket_agent_control,
                    &Protocol::NewClientRequest {
                        address: new_address,
                    },
                )
                .await
                .unwrap();

                println!("send protocol");
                let mut socket_agent = listener_for_agent.accept().await.unwrap().0;
                println!("accept");

                let protocol = read_protocol(&mut socket_agent_control).await.unwrap();

                assert_eq!(protocol, Protocol::NewClientResponse);
                println!("received new client response");

                tokio::spawn(async move {
                    let mut buf_client = [0; BUFFER_SIZE];
                    let mut buf_agent = [0; BUFFER_SIZE];

                    loop {
                        tokio::select! {
                            n = socket_client.read(&mut buf_client) => {
                                let n = match n {
                                    Ok(n) if n == 0 => return,
                                    Ok(n) => n,
                                    Err(e) => {
                                        eprintln!("failed to read from socket; err = {:?}", e);
                                        return;
                                    }
                                };
                                socket_agent.write_all(&buf_client[0..n]).await.unwrap();
                            }
                            n = socket_agent.read(&mut buf_agent) => {
                                let n = match n {
                                    Ok(n) if n == 0 => return,
                                    Ok(n) => n,
                                    Err(e) => {
                                        eprintln!("failed to read from socket; err = {:?}", e);
                                        return;
                                    }
                                };
                                socket_client.write_all(&buf_agent[0..n]).await.unwrap();
                            }
                        };
                    }
                });
            }
        });
    }
}
