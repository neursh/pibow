use defmt::*;
use embassy_executor::Spawner;
use embassy_net::{ Config, Stack, StackResources };
use embassy_net_wiznet::Device;
use embassy_rp::clocks::RoscRng;
use static_cell::StaticCell;

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) {
    runner.run().await
}

pub async fn invoke(spawner: Spawner, net_device: Device<'static>) -> Stack<'static> {
    let config = Config::dhcpv4(Default::default());
    let seed = RoscRng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<6>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed
    );

    unwrap!(spawner.spawn(net_task(runner)));

    stack
}
