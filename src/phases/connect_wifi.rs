use cyw43::{ Control, JoinOptions };
use embassy_net::Stack;
use embassy_time::Timer;

use crate::{ consts::*, phases::board };

pub async fn invoke(control: &mut Control<'static>, stack: &Stack<'static>) {
    // Connect to Wifi.
    board::serial_log("Joining wifi...");
    loop {
        match control.join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes())).await {
            Ok(_) => {
                break;
            }
            Err(_) => {
                board::serial_log("Can't join the Wifi network");
            }
        }
    }

    board::serial_log("Waiting for DHCP...");
    while !stack.is_config_up() || !stack.is_link_up() {
        Timer::after_millis(100).await;
    }
    board::serial_log("DHCP is now up!");
    // And now we can use the wifi!
}
