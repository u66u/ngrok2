use ngrok2::{SyncErrResult, HANDSHAKE, SERVER, SERVER_PORT};
use std::env::args;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::{io, task};

#[tokio::main]
async fn main() -> SyncErrResult {
    match args().nth(1).map(|a| a.parse()) {
        Some(Ok(local_port)) => {
            let mut server = TcpStream::connect((SERVER, SERVER_PORT)).await?;
            server.write_u128(HANDSHAKE).await?;
            let server_port = server.read_u16().await?;
            println!("working fine!");
            println!("{}:{} -> localhost:{}", SERVER, server_port, local_port);
            loop {
                let id = server.read_u128().await?;
                task::spawn(handle(id, local_port));
            }
        }
        Some(Err(_)) => println!("invalid port"),
        None => println!("usage: {} (port)", args().next().unwrap()),
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
