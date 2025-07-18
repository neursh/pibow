use core::net::{ Ipv4Addr, SocketAddr, SocketAddrV4 };

use embassy_net::{ udp::{ PacketMetadata, UdpSocket }, Stack };
use embassy_time::Timer;

use crate::consts::{ MULTICAST_IP, MULTICAST_PORT, NODE_PORT, SECRET_HASH_KEY, STACK_BUFFER_SIZE };

pub async fn invoke(stack: Stack<'static>, challenge: &[u8; 128]) {
    let mut rx_meta = [PacketMetadata::EMPTY; 8];
    let mut tx_meta = [PacketMetadata::EMPTY; 8];
    let mut rx_buffer = [0; STACK_BUFFER_SIZE];
    let mut tx_buffer = [0; STACK_BUFFER_SIZE];

    let mut announcer = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer
    );
    let _ = announcer.bind(NODE_PORT);

    blake3::keyed_hash(SECRET_HASH_KEY, &[1]);

    let multicast_addr = SocketAddr::V4(
        SocketAddrV4::new(Ipv4Addr::from_bits(MULTICAST_IP), MULTICAST_PORT)
    );

    let mut available_signal = [0_u8; 130];
    available_signal[0] = 69;
    available_signal[1] = 0;

    available_signal.copy_from_slice(challenge);

    loop {
        let _ = announcer.send_to(&available_signal, multicast_addr).await;
        Timer::after_secs(2).await;
    }
}
