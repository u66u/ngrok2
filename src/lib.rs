use dotenv::dotenv;
use std::collections::HashSet;
use std::env::var;
use std::error::Error;
pub const HANDSHAKE: u128 = u128::from_le_bytes(*b"123456789\0\0\0\0\0\0\0");
pub const SERVER_PORT: u16 = 3001;
pub const SERVER: &str = "..."; // server address here
pub const MAX_CONNECTIONS: usize = 100;

pub type ApiKey = String;

pub struct ClientInfo {
    pub api_key: ApiKey,
    // added for extensebility
}

pub enum AuthResponse {
    Success,
    Failure,
}

impl From<u8> for AuthResponse {
    fn from(value: u8) -> Self {
        match value {
            0 => AuthResponse::Failure,
            1 => AuthResponse::Success,
            _ => panic!("Invalid AuthResponse value"),
        }
    }
}

impl From<AuthResponse> for u8 {
    fn from(value: AuthResponse) -> Self {
        match value {
            AuthResponse::Failure => 0,
            AuthResponse::Success => 1,
        }
    }
}

pub fn init_api_keys() -> HashSet<ApiKey> {
    dotenv().ok();
    let keys_env =
        var("SERVER_VALID_API_KEYS").expect("Expected SERVER_VALID_API_KEYS in the environment");
    let keys_vec: Vec<ApiKey> = keys_env.split(',').map(|s| s.trim().to_string()).collect();

    keys_vec.into_iter().collect()
}

pub type SyncErrResult<T = ()> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
