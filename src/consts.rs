// Secret hash key must be shared with the server.
// Use build.py script to generate and obtain a random key.
pub const WIFI_NETWORK: &str = "ssid";
pub const WIFI_PASSWORD: &str = "password";
pub const SECRET_HASH_KEY: &[u8; 32] = &[0_u8; 32];

pub const CHALLENGE_LENGTH: usize = 32;

// The server poke destination.
pub const MULTICAST_IP: u32 = 3758096511; // 224.0.0.127
pub const MULTICAST_PORT: u16 = 4265;

// The port used on the node (this will open both TCP & UDP).
pub const NODE_PORT: u16 = 5325;

// The port used on server to let node connect to.
pub const SERVER_PORT: u16 = 7325;

// Fault tolerance from server before disconnecting for good.
pub const FAULT_TOLERANCE: usize = 10;

// The buffer for socket, not the receiving buffer for messages.
// All actions in here needs at most 150 bytes. Chose 512 for safety, that's all.
pub const STACK_BUFFER_SIZE: usize = 512;
