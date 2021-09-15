use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use structopt::StructOpt;
use tcp_tunnel_rs::Protocol::NewAgentResponse;
use tcp_tunnel_rs::{read_protocol, write_protocol, PortAssigner, PortRange, Protocol};
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

const BUFFER_SIZE: usize = 32 * 1024;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short = "c", long = "control_address", parse(try_from_str))]
    control_address: SocketAddr,
    #[structopt(
        short = "C",
        long = "client_port_range",
        parse(try_from_str),
        default_value = "10000:19999"
    )]
    client_port_range: PortRange,
    #[structopt(
        short = "a",
        long = "agent_port_range",
        parse(try_from_str),
        default_value = "20000:29999"
    )]
    tunnel_port_range: PortRange,
    #[structopt(short = "g", long = "myglobal_ip_address", parse(try_from_str))]
    myglobal_ip_address: IpAddr,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let listener_for_agent_control = TcpListener::bind(opt.control_address.clone()).await?;
    let tunnel_port_assigner = Arc::new(RwLock::new(PortAssigner::new(opt.tunnel_port_range)));
    let client_port_assigner = Arc::new(RwLock::new(PortAssigner::new(opt.client_port_range)));

    println!("start server");
    loop {
        let tunnel_port_assigner = tunnel_port_assigner.clone();
        let client_port_assigner = client_port_assigner.clone();
        let mut socket_agent_control = listener_for_agent_control.accept().await?.0;
        println!("connect agent");
        let new_client_port = client_port_assigner.write().await.next();
        let listener_for_client = TcpListener::bind(format!("0.0.0.0:{}", new_client_port)).await?;
        let myglobal_ip_address = opt.myglobal_ip_address.clone();

        write_protocol(
            &mut socket_agent_control,
            &NewAgentResponse {
                address: format!("{}:{}", &myglobal_ip_address, new_client_port),
            },
        )
        .await
        .unwrap();

        tokio::spawn(async move {
            loop {
                let (mut socket_client, _) = listener_for_client.accept().await.unwrap();
                println!("connect client");

                let new_port = tunnel_port_assigner.write().await.next();
                let new_address = format!("{}:{}", myglobal_ip_address.clone(), new_port);
                let listener_for_agent = TcpListener::bind(format!("0.0.0.0:{}", new_port))
                    .await
                    .unwrap();
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
