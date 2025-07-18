#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]

mod consts;
mod phases;

use embassy_executor::Spawner;
use embassy_rp::clocks::RoscRng;
use crate::{ consts::SECRET_HASH_KEY, phases::{ cast_node, connect_wifi, init_board, setup_wifi } };

use ::{ defmt_rtt as _, panic_probe as _ };

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

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
        for index in 2..130 {
            challenge[index] = RoscRng::next_u8();
        }
        let expected_answer = blake3::keyed_hash(SECRET_HASH_KEY, &challenge);

        cast_node::invoke(stack, &mut control, &challenge).await;
    }
}
