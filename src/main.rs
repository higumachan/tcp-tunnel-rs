use tokio;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const BUFFER_SIZE: usize = 32 * 1024;

#[tokio::main]
async fn main() {
    let listener_for_agent = TcpListener::bind("127.0.0.1:7136").await?;

    loop {
        let (mut socket_agent, _) = listener_for_agent.accept().await?;
        println!("connect agent");
        let listener_for_client = TcpListener::bind("127.0.0.1:7135").await?;
        tokio::spawn(async move {
            let (mut socket_client, _)  = listener_for_client.accept().await?;
            println!("connect client");

            tokio::spawn(async move {
                let mut buf = [0; BUFFER_SIZE];

                loop {
                    let n = match socket_client.read(&mut buf).await {
                        Ok(n) if n == 0 => return,
                        Ok(n) => n,
                        Err(e) => {
                            eprintln!("failed to read from socket; err = {:?}", e);
                            return;
                        }
                    };

                    if let Err(e) = socket_agent.write_all(&buf[0..n]).await {
                        eprintln!("failed to write to socket; err = {:?}", e);
                        return;
                    }
                }
            });
        });
    }
}
