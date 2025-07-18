use cyw43::{ Control, JoinOptions };
use defmt::*;
use embassy_net::Stack;
use embassy_time::Timer;

use crate::consts::*;

pub async fn invoke(control: &mut Control<'static>, stack: &Stack<'static>) {
    // Connect to Wifi.
    loop {
        match control.join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes())).await {
            Ok(_) => {
                break;
            }
            Err(err) => {
                info!("join failed with status={}", err.status);
            }
        }
    }

    // Wait for DHCP, not necessary when using static IP.
    info!("waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    info!("DHCP is now up!");
    // And now we can use the wifi!
}
