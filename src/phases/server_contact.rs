use blake3::Hash;
use embassy_futures::select::{ select, Either };
use embassy_net::{ tcp::{ self, TcpSocket }, IpAddress, IpEndpoint, Stack };
use embassy_rp::{ clocks::RoscRng, gpio::{ Input, Level, Output } };
use embassy_time::Timer;
use embedded_io_async::{ Read, ReadExactError, Write };

use crate::{
    consts::{
        ACTIVATE_RELAY,
        ANSWER_LENGTH,
        CHALLENGE_LENGTH,
        DEACTIVATE_RELAY,
        FAULT_TOLERANCE,
        SECRET_HASH_KEY,
        SERVER_PORT,
        STACK_BUFFER_SIZE,
    },
    phases::board,
};

pub async fn invoke(
    stack: Stack<'static>,
    server_address: IpAddress,
    mac_address: [u8; 6],
    power_switch: &mut Output<'static>,
    reset_switch: &mut Output<'static>,
    machine_state: &mut Input<'static>
) {
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

    let (mut reader, mut writer) = socket.split();

    // Enclose this whole initial communication, don't wanna waste memory keeping this for no reason.
    {
        // Read the challenge.
        let mut challenge = [0_u8; CHALLENGE_LENGTH];
        if let Err(_) = reader.read_exact(&mut challenge).await {
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

        if let Err(_) = writer.write_all(&introduce_with_answer).await {
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

    let mut reported_state: Option<Level> = None;

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
        // Notifying the server that this is a challenge.
        if let Err(_) = writer.write_all(&[2]).await {
            board::serial_log("Can't send the notifier to server, breaking...");
            break;
        }
        if let Err(_) = writer.write_all(&current_challenge).await {
            board::serial_log("Can't send the challenge to server, breaking...");
            break;
        }

        // The longest waiting task that we can attach to is wait for action & answer from server.
        // So in the mean time, we can use this wait time to send report of the machine's state to server.
        // Since we can't move mutable across states without unsafe, this is the best way that I could think of.
        let bad_cases: Either<
            Result<(), ReadExactError<tcp::Error>>,
            Result<(), tcp::Error>
        > = select(
            // Wait for action & answer.
            (async || {
                if let Err(bad) = reader.read_exact(&mut action_with_answer).await {
                    board::serial_log("Can't obtain action & answer from server, breaking...");
                    return Err(bad);
                }
                Ok(())
            })(),
            // Watch for machine's state.
            (async || {
                let mut current_state: Level;
                loop {
                    if let Some(state) = reported_state {
                        // Already reported, wait for a new one.
                        if machine_state.get_level() == state {
                            machine_state.wait_for_any_edge().await;
                        }
                    }

                    // Store new state to variable.
                    current_state = machine_state.get_level();
                    reported_state = Some(current_state);

                    // Check and send the new state.
                    let write_state: u8;
                    if current_state == Level::High {
                        write_state = 1;
                    } else {
                        write_state = 0;
                    }
                    if let Err(bad) = writer.write(&[write_state]).await {
                        board::serial_log("Can't obtain action & answer from server, breaking...");
                        return Err(bad);
                    }
                }
            })()
        ).await;

        match bad_cases {
            Either::First(bad) => if bad.is_err() {
                break;
            }
            Either::Second(bad) => if bad.is_err() {
                break;
            }
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

        // Get action.
        let action = action_with_answer[0];

        // Execute the action.
        // Power on.
        if action == 1 {
            if machine_state.get_level() == Level::Low {
                power_switch.set_level(ACTIVATE_RELAY);
                Timer::after_millis(500).await;
                power_switch.set_level(DEACTIVATE_RELAY);
            }
            // No don't press it when it's already on.
            if machine_state.get_level() == Level::High {
                // Send back already on state.
                if let Err(_) = writer.write(&[1]).await {
                    board::serial_log("Can't obtain action & answer from server, breaking...");
                    break;
                }
            }
            continue;
        }
        // Power off.
        if action == 2 {
            if machine_state.get_level() == Level::High {
                power_switch.set_level(ACTIVATE_RELAY);
                Timer::after_millis(500).await;
                power_switch.set_level(DEACTIVATE_RELAY);
            }
            // No don't press it when it's already off.
            if machine_state.get_level() == Level::Low {
                // Send back already on state.
                if let Err(_) = writer.write(&[1]).await {
                    board::serial_log("Can't obtain action & answer from server, breaking...");
                    break;
                }
            }
            continue;
        }
        // Reset
        if action == 3 {
            reset_switch.set_low();
            Timer::after_millis(500).await;
            reset_switch.set_high();
        }
    }

    power_switch.set_high();
    reset_switch.set_high();
    let _ = socket.flush().await;
    socket.abort();
    socket.close();
}
