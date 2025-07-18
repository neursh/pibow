use blake3::Hash;
use cyw43::Control;
use embassy_net::{ tcp::TcpSocket, Stack };
use embassy_sync::{ blocking_mutex::raw::CriticalSectionRawMutex, channel::Sender };
use embassy_time::Duration;
use embedded_io_async::Read;

use crate::consts::{ NODE_PORT, STACK_BUFFER_SIZE };

pub async fn invoke(
    stack: Stack<'static>,
    control: &Control<'static>,
    expected_answer: Hash,
    poke_destroyer_sender: Sender<'_, CriticalSectionRawMutex, bool, 1>
) {
    let mut rx_buffer = [0; STACK_BUFFER_SIZE];
    let mut tx_buffer = [0; STACK_BUFFER_SIZE];

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

    loop {
        socket.set_timeout(Some(Duration::from_secs(10)));
        if let Err(_) = socket.accept(NODE_PORT).await {
            continue;
        }

        let mut challenge_answer = [0_u8; 34];

        if let Err(_) = socket.read_exact(&mut challenge_answer).await {
            let _ = socket.flush().await;
            socket.abort();
            socket.close();
            continue;
        }

        let answer_hash = Hash::from_bytes(challenge_answer[2..34].try_into().unwrap());

        if challenge_answer[0..2] != [69, 1] || expected_answer != answer_hash {
            let _ = socket.flush().await;
            socket.abort();
            socket.close();
            continue;
        }

        // Stop the whole thang, don't look any further.
        // From this point on, any error will break this loop, render this node back to poke server mode.
        poke_destroyer_sender.send(true).await;
    }
}
