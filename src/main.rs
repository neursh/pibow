#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]

mod consts;
mod phases;

use embassy_executor::Spawner;
use embassy_futures::{ join::join, select::select };
use embassy_rp::clocks::RoscRng;
use embassy_sync::{ blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel };
use crate::{
    consts::SECRET_HASH_KEY,
    phases::{ connect_wifi, init_board, node_control, poke_server, setup_wifi },
};

use ::{ defmt_rtt as _, panic_probe as _ };

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let peripherals = embassy_rp::init(Default::default());

    // Initialize the board.
    let (mut control, net_device) = init_board::invoke(spawner, peripherals).await;

    // Initialize the Wifi stack.
    let stack = setup_wifi::invoke(spawner, net_device).await;

    // Board launched, light up.
    control.gpio_set(0, true).await;

    loop {
        // Conenct to the Wifi.
        connect_wifi::invoke(&mut control, &stack).await;

        // Create a hash challenge and cast it to the UDP channel.
        let mut challenge = [0_u8; 128];
        for index in 0..128 {
            challenge[index] = RoscRng::next_u8();
        }
        let expected_answer = blake3::keyed_hash(SECRET_HASH_KEY, &challenge);
        let poke_destroyer: Channel<CriticalSectionRawMutex, bool, 1> = Channel::new();

        let poke_destroyer_recv = poke_destroyer.receiver();
        let poke_destroyer_send = poke_destroyer.sender();

        let poke_server = select(
            poke_destroyer_recv.receive(),
            poke_server::invoke(stack, &challenge)
        );

        join(
            poke_server,
            node_control::invoke(stack, &control, expected_answer, poke_destroyer_send)
        ).await;
    }
}
