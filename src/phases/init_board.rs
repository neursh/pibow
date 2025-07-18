use cyw43::Control;
use cyw43_pio::{ PioSpi, DEFAULT_CLOCK_DIVIDER };
use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_net_wiznet::Device;
use embassy_rp::{
    bind_interrupts,
    gpio::{ Level, Output },
    peripherals::{ DMA_CH0, PIO0 },
    pio::{ InterruptHandler, Pio },
    Peripherals,
};
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>
) {
    runner.run().await
}

pub async fn invoke(spawner: Spawner, peripherals: Peripherals) -> (Control<'static>, Device<'static>) {
    // Load Wifi chip driver.
    let fw = include_bytes!("../../embassy/cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../../embassy/cyw43-firmware/43439A0_clm.bin");

    let pwr = Output::new(peripherals.PIN_23, Level::Low);
    let cs = Output::new(peripherals.PIN_25, Level::High);
    let mut pio = Pio::new(peripherals.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        peripherals.PIN_24,
        peripherals.PIN_29,
        peripherals.DMA_CH0
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(cyw43_task(runner)));

    control.init(clm).await;
    control.set_power_management(cyw43::PowerManagementMode::PowerSave).await;

    (control, net_device)
}
