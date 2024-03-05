use ngrok2::{SyncErrResult, HANDSHAKE, MAX_CONNECTIONS, SERVER_PORT};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::{io, task, time};
use tracing::{debug, error, info};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::prelude::*;

#[derive(Default)]
struct State {
    connections: HashMap<u128, TcpStream>,
    max_connections: usize,
}

async fn check_max_connections(state: Arc<Mutex<State>>) -> bool {
    let state = state.lock().await;
    state.connections.len() < state.max_connections
}

#[tokio::main]
async fn main() -> SyncErrResult {
    tracing_subscriber::registry()
        .with(console_subscriber::spawn())
        .with(tracing_subscriber::fmt::layer().with_filter(LevelFilter::DEBUG))
        .init();
    let state = Arc::new(Mutex::new(State {
        connections: HashMap::new(),
        max_connections: MAX_CONNECTIONS,
    }));
    let listener = TcpListener::bind(("0.0.0.0", SERVER_PORT)).await?;
    info!("Started server on {}", SERVER_PORT);
    while let Ok((stream, _)) = time::timeout(Duration::from_secs(30), listener.accept()).await? {
        let state = state.clone();
        task::spawn(async move {
            if let Err(e) = handle(state, stream).await {
                error!("{}", e);
            }
        });
    }
    Ok(())
}

async fn handle(state: Arc<Mutex<State>>, mut client: TcpStream) -> SyncErrResult {
    match client.read_u128().await? {
        HANDSHAKE => {
            if !check_max_connections(state.clone()).await {
                client.write_u16(0).await?;
                info!("Connection rejected: maximum number of connections reached");
                return Ok(());
            }

            let listener = TcpListener::bind("0.0.0.0:0").await.unwrap(); // use random available port
            let port = listener.local_addr()?.port();
            let ip = client.peer_addr()?;
            info!("{} connected on {}", ip, port);
            client.write_u16(port).await?;
            let (mut read, mut write) = client.into_split();
            let listen = task::spawn(async move {
                while let Ok((stream, _)) = listener.accept().await {
                    let id = rand::random();
                    state.lock().await.connections.insert(id, stream);
                    write.write_u128(id).await.unwrap();
                    write.flush().await.unwrap();
                    task::spawn(delete(state.clone(), id));
                }
            });
            let _ = read.read_u8().await;
            info!("{} disconnected", ip);
            listen.abort();
        }
        id => {
            let conn = state.lock().await.connections.remove(&id);
            if let Some(mut conn) = conn {
                io::copy_bidirectional(&mut client, &mut conn).await?;
            }
        }
    }
    Ok(())
}

async fn delete(state: Arc<Mutex<State>>, id: u128) {
    time::sleep(Duration::from_secs(10)).await;
    if state.lock().await.connections.remove(&id).is_some() {
        debug!("Removed stale connection {}", id);
    }
}
