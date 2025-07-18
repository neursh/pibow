use core::net::{ Ipv4Addr, SocketAddr, SocketAddrV4 };

use cyw43::Control;
use embassy_net::{ udp::{ PacketMetadata, UdpSocket }, Stack };
use embassy_time::Timer;

use crate::consts::SECRET_HASH_KEY;

pub async fn invoke(stack: Stack<'static>, control: &mut Control<'static>, challenge: &[u8; 128]) {
    let mut rx_meta = [PacketMetadata::EMPTY; 8];
    let mut tx_meta = [PacketMetadata::EMPTY; 8];
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    let mut announcer = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer
    );
    let _ = announcer.bind(5325);

    blake3::keyed_hash(SECRET_HASH_KEY, &[1]);

    let multicast_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(224, 0, 0, 127), 4265));

    let mut available_signal = [0_u8; 130];
    available_signal[0] = 69;
    available_signal[1] = 0;

    available_signal.copy_from_slice(challenge);

    loop {
        control.gpio_set(0, true).await;
        let _ = announcer.send_to(&available_signal, multicast_addr).await;
        Timer::after_secs(1).await;
        control.gpio_set(0, false).await;
        Timer::after_secs(1).await;
    }
}
