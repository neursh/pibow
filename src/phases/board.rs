use cyw43::Control;
use cyw43_pio::{ PioSpi, DEFAULT_CLOCK_DIVIDER };
use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_net_wiznet::Device;
use embassy_rp::{
    bind_interrupts,
    gpio::{ Level, Output },
    peripherals::{ DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO0, USB },
    pio::{ self, Pio },
    usb::{ self, Driver },
    Peri,
};
use embassy_sync::{ blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel };
use embassy_usb::class::cdc_acm::{ CdcAcmClass, State };
use embassy_usb::{ Builder, Config };
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>
) {
    runner.run().await
}

#[embassy_executor::task]
async fn usb_task(mut usb: embassy_usb::UsbDevice<'static, Driver<'static, USB>>) -> ! {
    usb.run().await
}

// Channel for sending messages to serial logger
static SERIAL_CHANNEL: Channel<CriticalSectionRawMutex, heapless::String<256>, 8> = Channel::new();

#[embassy_executor::task]
async fn serial_logger_task(mut class: CdcAcmClass<'static, Driver<'static, USB>>) {
    loop {
        class.wait_connection().await;

        loop {
            // Wait for messages from the channel or send heartbeat
            match
                embassy_time::with_timeout(
                    embassy_time::Duration::from_secs(5),
                    SERIAL_CHANNEL.receive()
                ).await
            {
                Ok(msg) => {
                    let mut buffer = [0u8; 258];
                    let msg_bytes = msg.as_bytes();
                    let len = msg_bytes.len().min(256);
                    buffer[..len].copy_from_slice(&msg_bytes[..len]);
                    buffer[len] = b'\r';
                    buffer[len + 1] = b'\n';

                    match class.write_packet(&buffer[..len + 2]).await {
                        Ok(_) => {}
                        Err(embassy_usb::driver::EndpointError::BufferOverflow) => {}
                        Err(embassy_usb::driver::EndpointError::Disabled) => {
                            break;
                        }
                    }
                }
                Err(_) => {
                    // Timeout - send heartbeat
                    let heartbeat = b"1\r\n";
                    match class.write_packet(heartbeat).await {
                        Ok(_) => {}
                        Err(embassy_usb::driver::EndpointError::Disabled) => {
                            break;
                        }
                        Err(_) => {}
                    }
                }
            }
        }
    }
}

// Helper function to send messages to serial logger
pub fn serial_log(msg: &str) {
    if let Ok(string_msg) = heapless::String::try_from(msg) {
        let _ = SERIAL_CHANNEL.try_send(string_msg);
    }
}

pub async fn initialize(
    spawner: Spawner,
    pins: (
        Peri<'static, PIN_23>,
        Peri<'static, PIN_24>,
        Peri<'static, PIN_25>,
        Peri<'static, PIN_29>,
        Peri<'static, PIO0>,
        Peri<'static, DMA_CH0>,
    ),
    usb: Peri<'static, USB>
) -> (Control<'static>, Device<'static>) {
    init_usb(spawner, usb).await;
    init_wifi(spawner, pins).await
}

async fn init_usb(spawner: Spawner, usb_port: Peri<'static, USB>) {
    let driver = Driver::new(usb_port, Irqs);
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Neurs");
    config.product = Some("Pibow Debug Interface");
    config.serial_number = Some("347245734");
    config.max_power = 100;
    config.max_packet_size_0 = 64;
    config.device_class = 0xef;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    static DEVICE_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static CONFIG_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

    let device_desc = DEVICE_DESC.init([0; 256]);
    let config_desc = CONFIG_DESC.init([0; 256]);
    let bos_desc = BOS_DESC.init([0; 256]);
    let control_buf = CONTROL_BUF.init([0; 64]);

    let mut builder = Builder::new(driver, config, device_desc, config_desc, bos_desc, control_buf);

    static USB_STATE: StaticCell<State> = StaticCell::new();
    let usb_state = USB_STATE.init(State::new());
    let serial_class = CdcAcmClass::new(&mut builder, usb_state, 64);
    let usb_device = builder.build();

    unwrap!(spawner.spawn(usb_task(usb_device)));
    unwrap!(spawner.spawn(serial_logger_task(serial_class)));
}

async fn init_wifi(
    spawner: Spawner,
    pins: (
        Peri<'static, PIN_23>,
        Peri<'static, PIN_24>,
        Peri<'static, PIN_25>,
        Peri<'static, PIN_29>,
        Peri<'static, PIO0>,
        Peri<'static, DMA_CH0>,
    )
) -> (Control<'static>, Device<'static>) {
    let wifi_firmware = include_bytes!("../../embassy/cyw43-firmware/43439A0.bin");
    let wifi_country_locale = include_bytes!("../../embassy/cyw43-firmware/43439A0_clm.bin");
    let wifi_power_pin = Output::new(pins.0, Level::Low);
    let spi_chip_select = Output::new(pins.2, Level::High);
    let mut programmable_io = Pio::new(pins.4, Irqs);
    let spi_interface = PioSpi::new(
        &mut programmable_io.common,
        programmable_io.sm0,
        DEFAULT_CLOCK_DIVIDER,
        programmable_io.irq0,
        spi_chip_select,
        pins.1,
        pins.3,
        pins.5
    );
    static WIFI_STATE: StaticCell<cyw43::State> = StaticCell::new();
    let wifi_state = WIFI_STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(
        wifi_state,
        wifi_power_pin,
        spi_interface,
        wifi_firmware
    ).await;
    unwrap!(spawner.spawn(cyw43_task(runner)));
    control.init(wifi_country_locale).await;
    control.set_power_management(cyw43::PowerManagementMode::PowerSave).await;

    (control, net_device)
}
