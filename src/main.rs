#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]

mod consts;
mod phases;

use core::panic::PanicInfo;

use embassy_executor::Spawner;
use embassy_futures::{ join::join, select::select };
use embassy_rp::clocks::RoscRng;
use embassy_sync::{ blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel };
use crate::{
    consts::{ CHALLENGE_LENGTH, SECRET_HASH_KEY },
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

    loop {
        // Add a cancel poke channel.
        let cancel_poke: Channel<CriticalSectionRawMutex, bool, 1> = Channel::new();
        let cancel_poke_recv = cancel_poke.receiver();
        let cancel_poke_send = cancel_poke.sender();

        // Create a hash challenge and cast it to the UDP channel.
        let mut challenge = [0_u8; CHALLENGE_LENGTH];
        for index in 0..CHALLENGE_LENGTH {
            challenge[index] = RoscRng::next_u8();
        }
        let expected_answer = blake3::keyed_hash(SECRET_HASH_KEY, &challenge);

        // Create a UDP multicast socket to poke the server.
        // Since the poke server will run forever, the cancel channel is used to drop this task.
        // It will be dropped by accept_answer when a good server contacted it.
        let poke_server = select(
            cancel_poke_recv.receive(),
            poke_server::invoke(stack, &challenge)
        );

        // Receive the remote address of the server. We will then connect back to this under a defined port.
        let (_, server_address) = join(
            poke_server,
            listen_answer::invoke(stack, expected_answer, cancel_poke_send)
        ).await;

        // Found connection, light up!
        control.gpio_set(0, true).await;

        server_contact::invoke(stack, server_address, mac_address).await;
    }
}
