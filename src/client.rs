use dotenv::dotenv;
use ngrok2::{ApiKey, AuthResponse, SyncErrResult, HANDSHAKE, SERVER, SERVER_PORT};
use std::env::{args, var};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::{io, task};

#[tokio::main]
async fn main() -> SyncErrResult {
    dotenv().ok();
    match args().nth(1).map(|a| a.parse()) {
        Some(Ok(local_port)) => {
            let api_key: ApiKey =
                var("CLIENT_API_KEY").expect("Expected CLIENT_API_KEY in the environment");

            let mut server = TcpStream::connect((SERVER, SERVER_PORT)).await?;

            server.write_all(api_key.as_bytes()).await?;
            server.flush().await?;

            let response = server.read_u8().await?;

            match AuthResponse::from(response) {
                AuthResponse::Success => {
                    server.write_u128(HANDSHAKE).await?;
                    let server_port = server.read_u16().await?;
                    println!("Connected to server successfully!");
                    println!(
                        "Your application is accessible at: {}:{} (forwarded from localhost:{})",
                        SERVER, server_port, local_port
                    );
                    loop {
                        let id = server.read_u128().await?;
                        task::spawn(handle(id, local_port));
                    }
                }
                AuthResponse::Failure => {
                    println!("Authentication failed");
                    return Err("Authentication failed".into());
                }
            }
        }
        Some(Err(_)) => println!("Invalid port"),
        None => println!("Usage: {} <local_port>", args().next().unwrap()),
    };
    Ok(())
}

async fn handle(id: u128, local_port: u16) -> SyncErrResult {
    let mut stream = TcpStream::connect((SERVER, SERVER_PORT)).await?;
    stream.write_u128(id).await?;
    stream.flush().await?;
    io::copy_bidirectional(
        &mut stream,
        &mut TcpStream::connect(("0.0.0.0", local_port)).await?,
    )
    .await?;
    Ok(())
}
