use blake3::Hash;
use embassy_net::{ tcp::TcpSocket, IpAddress, Stack };
use embassy_time::Duration;
use embedded_io_async::Read;

use crate::{ consts::{ ANSWER_LENGTH, NODE_PORT, STACK_BUFFER_SIZE }, phases::board };

pub async fn invoke(stack: Stack<'static>, expected_answer: Hash) -> IpAddress {
    let mut rx_buffer = [0_u8; STACK_BUFFER_SIZE];
    let mut tx_buffer = [0_u8; STACK_BUFFER_SIZE];

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(2)));

    board::serial_log("TCP Initialized");

    loop {
        if let Err(_) = socket.accept(NODE_PORT).await {
            continue;
        }

        let mut challenge_answer = [0_u8; ANSWER_LENGTH];

        if let Err(_) = socket.read_exact(&mut challenge_answer).await {
            let _ = socket.flush().await;
            socket.abort();
            socket.close();
            continue;
        }

        let answer_hash = Hash::from_bytes(challenge_answer);

        if expected_answer != answer_hash {
            let _ = socket.flush().await;
            socket.abort();
            socket.close();
            continue;
        }

        let remote_endpoint = socket.remote_endpoint().unwrap();

        // Disconnect the server. We'll connect to it.
        let _ = socket.flush().await;
        socket.abort();
        socket.close();

        return remote_endpoint.addr;
    }
}
