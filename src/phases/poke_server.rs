use core::net::{ Ipv4Addr, SocketAddr, SocketAddrV4 };

use embassy_net::{ udp::{ PacketMetadata, UdpSocket }, Stack };
use embassy_time::Timer;

use crate::{
    consts::{ CHALLENGE_LENGTH, MULTICAST_IP, MULTICAST_PORT, NODE_PORT, STACK_BUFFER_SIZE },
    phases::board,
};

pub async fn invoke(stack: Stack<'static>, challenge: &[u8; CHALLENGE_LENGTH]) {
    let mut rx_meta = [PacketMetadata::EMPTY; 8];
    let mut rx_buffer = [0; STACK_BUFFER_SIZE];
    let mut tx_meta = [PacketMetadata::EMPTY; 8];
    let mut tx_buffer = [0; STACK_BUFFER_SIZE];

    let mut announcer = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer
    );
    let _ = announcer.bind(NODE_PORT);

    board::serial_log("UDP Initialized");

    let multicast_addr = SocketAddr::V4(
        SocketAddrV4::new(Ipv4Addr::from_bits(MULTICAST_IP), MULTICAST_PORT)
    );

    loop {
        let _ = announcer.send_to(challenge, multicast_addr).await;
        Timer::after_secs(2).await;
    }
}
