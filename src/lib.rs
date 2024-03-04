use std::error::Error;

pub const HANDSHAKE: u128 = u128::from_le_bytes(*b"123456789\0\0\0\0\0\0\0");
pub const SERVER_PORT: u16 = 3001;
pub const SERVER: &str = "..."; // server address here
                                //
pub type Result<T = ()> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
