#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]

mod consts;
mod phases;

use core::panic::PanicInfo;

use embassy_executor::Spawner;
use embassy_futures::select::{ select, Either };
use embassy_rp::{ clocks::RoscRng, gpio::{ Input, Output, Pull } };
use crate::{
    consts::{ CHALLENGE_LENGTH, DEACTIVATE_RELAY, SECRET_HASH_KEY },
    phases::{ board, connect_wifi, listen_answer, poke_server, server_contact, setup_stack },
};

use ::{ defmt_rtt as _ };

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    cortex_m::peripheral::SCB::sys_reset();
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let peripherals = embassy_rp::init(Default::default());

    // Initialize the board.
    let (mut control, net_device) = board::initialize(
        spawner,
        (
            peripherals.PIN_23,
            peripherals.PIN_24,
            peripherals.PIN_25,
            peripherals.PIN_29,
            peripherals.PIO0,
            peripherals.DMA_CH0,
        ),
        peripherals.USB
    ).await;

    // Initialize the Wifi stack.
    let stack = setup_stack::invoke(spawner, net_device).await;

    // Conenct to the Wifi.
    connect_wifi::invoke(&mut control, &stack).await;

    let mac_address = control.address().await;

    let mut power_switch = Output::new(peripherals.PIN_14, DEACTIVATE_RELAY);
    let mut reset_switch = Output::new(peripherals.PIN_15, DEACTIVATE_RELAY);
    let mut machine_state = Input::new(peripherals.PIN_16, Pull::Down);

    loop {
        // Create a hash challenge and cast it to the UDP channel.
        let mut challenge = [0_u8; CHALLENGE_LENGTH];
        for index in 0..CHALLENGE_LENGTH {
            challenge[index] = RoscRng::next_u8();
        }
        let expected_answer = blake3::keyed_hash(SECRET_HASH_KEY, &challenge);

        // Create a UDP multicast socket to poke the server.
        // It will be dropped by executor after listen_answer was selected when a good server contacted it.
        // In case the poke_server finishes first, just redo this process.
        // Receive the remote address of the server. We will then connect back to this under a defined port.
        let expect_server_address = select(
            poke_server::invoke(stack, &challenge),
            listen_answer::invoke(stack, expected_answer)
        ).await;

        let server_address = match expect_server_address {
            Either::First(_) => {
                continue;
            }
            Either::Second(server_address) => server_address,
        };

        // Found connection, light up!
        control.gpio_set(0, true).await;

        server_contact::invoke(
            stack,
            server_address,
            mac_address,
            &mut power_switch,
            &mut reset_switch,
            &mut machine_state
        ).await;
    }
}
