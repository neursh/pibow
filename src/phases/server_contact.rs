use blake3::Hash;
use embassy_net::{ tcp::TcpSocket, IpAddress, IpEndpoint, Stack };
use embassy_rp::clocks::RoscRng;
use embedded_io_async::{ Read, Write };

use crate::{
    consts::{
        CHALLENGE_LENGTH,
        FAULT_TOLERANCE,
        ANSWER_LENGTH,
        SECRET_HASH_KEY,
        SERVER_PORT,
        STACK_BUFFER_SIZE,
    },
    phases::board,
};

pub async fn invoke(stack: Stack<'static>, server_address: IpAddress, mac_address: [u8; 6]) {
    let mut rx_buffer = [0_u8; STACK_BUFFER_SIZE];
    let mut tx_buffer = [0_u8; STACK_BUFFER_SIZE];

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

    if let Err(_) = socket.connect(IpEndpoint::new(server_address, SERVER_PORT)).await {
        board::serial_log("Can't connect to server endpoint");
        let _ = socket.flush().await;
        socket.abort();
        socket.close();
        return;
    }

    // Enclose this whole initial communication, don't wanna waste memory keeping this for no reason.
    {
        // Read the challenge.
        let mut challenge = [0_u8; CHALLENGE_LENGTH];
        if let Err(_) = socket.read_exact(&mut challenge).await {
            board::serial_log("Can't obtain the challenge from server.");
            let _ = socket.flush().await;
            socket.abort();
            socket.close();
            return;
        }

        // Answer the challenge, also introduce this node.
        let answer = blake3::keyed_hash(SECRET_HASH_KEY, &challenge);
        let mut introduce_with_answer = [0_u8; 38];
        introduce_with_answer[0..6].copy_from_slice(&mac_address);
        introduce_with_answer[6..38].copy_from_slice(answer.as_bytes());

        if let Err(_) = socket.write_all(&introduce_with_answer).await {
            board::serial_log("Can't introduce to the server, folding...");
            let _ = socket.flush().await;
            socket.abort();
            socket.close();
            return;
        }
    }

    // If nothing goes wrong, start taking requests from server!
    let mut current_challenge = [0_u8; CHALLENGE_LENGTH];
    let mut expected_answer: Hash;
    let mut action_with_answer = [0_u8; ANSWER_LENGTH + 1];

    // Counter on how many faults from the server.
    let mut faults: usize = 0;

    loop {
        // Check faults.
        if faults > FAULT_TOLERANCE {
            break;
        }

        // Prepare the challenge.
        for index in 0..CHALLENGE_LENGTH {
            current_challenge[index] = RoscRng::next_u8();
        }
        expected_answer = blake3::keyed_hash(SECRET_HASH_KEY, &current_challenge);
        // Send challenge.
        if let Err(_) = socket.write_all(&current_challenge).await {
            board::serial_log("Can't send the challenge to server, breaking...");
            break;
        }

        // Wait for action input & answer.
        if let Err(_) = socket.read_exact(&mut action_with_answer).await {
            board::serial_log("Can't obtain action & answer from server, breaking...");
            break;
        }

        let hash_answer = Hash::from_bytes(
            action_with_answer[1..ANSWER_LENGTH + 1].try_into().unwrap()
        );

        // Compare hashes.
        if expected_answer != hash_answer {
            board::serial_log("Server failed the challenge, folding...");
            faults += 1;
            continue;
        }

        let action = action_with_answer[0];
    }

    let _ = socket.flush().await;
    socket.abort();
    socket.close();
}
