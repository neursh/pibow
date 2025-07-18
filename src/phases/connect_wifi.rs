use cyw43::{ Control, JoinOptions };
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::{ Config, Stack, StackResources };
use embassy_net_wiznet::Device;
use embassy_rp::{ clocks::RoscRng };
use embassy_time::Timer;
use static_cell::StaticCell;

use crate::consts::*;

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) {
    runner.run().await
}

pub async fn invoke(
    spawner: Spawner,
    control: &mut Control<'static>,
    net_device: Device<'static>
) -> Stack<'static> {
    let config = Config::dhcpv4(Default::default());
    let seed = RoscRng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed
    );

    // Connect to Wifi.
    unwrap!(spawner.spawn(net_task(runner)));
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

    stack
}
