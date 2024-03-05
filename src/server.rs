use ngrok2::{
    init_api_keys, ApiKey, AuthResponse, SyncErrResult, HANDSHAKE, MAX_CONNECTIONS, SERVER_PORT,
};
use std::collections::{HashMap, HashSet};
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

    let api_keys = init_api_keys();

    while let Ok((stream, _)) = time::timeout(Duration::from_secs(30), listener.accept()).await? {
        let state = state.clone();
        let api_keys = api_keys.clone();
        task::spawn(async move {
            if let Err(e) = handle(state, stream, api_keys).await {
                error!("{}", e);
            }
        });
    }
    Ok(())
}

async fn handle(
    state: Arc<Mutex<State>>,
    mut client: TcpStream,
    api_keys: HashSet<ApiKey>,
) -> SyncErrResult {
    let mut api_key = [0; 256];
    client.read_exact(&mut api_key).await?;

    let api_key = String::from_utf8_lossy(&api_key)
        .trim_end_matches('\0')
        .to_string();

    if !api_keys.contains(&api_key) {
        client.write_u8(AuthResponse::Failure as u8).await?;
        client.flush().await?;
        return Err("Authentication failed".into());
    }

    client.write_u8(AuthResponse::Success as u8).await?;
    client.flush().await?;

    match client.read_u128().await? {
        HANDSHAKE => {
            if !check_max_connections(state.clone()).await {
                client.write_u16(0).await?;
                info!("Connection rejected: maximum number of connections reached");
                return Err("Maximum number of connections reached".into());
            }

            let listener = TcpListener::bind("0.0.0.0:0").await?;
            let server_port = listener.local_addr()?.port();

            let ip = client.peer_addr()?;
            info!("{} connected on {}", ip, server_port);
            client.write_u16(server_port).await?;

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
