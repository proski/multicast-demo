use anyhow::Result;
use socket2::{Domain, Socket, Type};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{str, thread};
use tracing::info;

const RX_ADDRESS: Ipv4Addr = Ipv4Addr::new(127, 0, 123, 1);
const TX_ADDRESS: Ipv4Addr = Ipv4Addr::new(127, 0, 123, 2);
const PORT: u16 = 12345;
const RX_SOCKADDR: SocketAddr = SocketAddr::new(IpAddr::V4(RX_ADDRESS), PORT);
const TX_SOCKADDR: SocketAddr = SocketAddr::new(IpAddr::V4(TX_ADDRESS), 0u16);

fn receiver1(running: Arc<AtomicBool>) -> Result<()> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
    socket.set_reuse_address(true)?;
    socket.set_multicast_all_v4(false)?;
    socket.set_read_timeout(Some(Duration::from_millis(50)))?;
    socket.bind(&RX_SOCKADDR.into())?;
    for i in 1..3 {
        let multicast_addr: Ipv4Addr = format!("239.0.0.{i}").as_str().parse()?;
        info!(?multicast_addr, "receiver 1 joining");
        socket.join_multicast_v4(&multicast_addr, &RX_ADDRESS)?;
    }

    let socket: UdpSocket = socket.into();
    let mut buffer = vec![0_u8; 65536];

    while running.load(Ordering::Relaxed) {
        let Ok((packet_size, sockaddr)) = socket.recv_from(&mut buffer) else {
            continue;
        };
        let data = str::from_utf8(&buffer[0..packet_size])?;
        info!(?data, ?packet_size, ?sockaddr, "receiver 1 received");
    }
    Ok(())
}

fn receiver2(_running: Arc<AtomicBool>) -> Result<()> {
    Ok(())
}

fn sender() -> Result<()> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
    socket.bind(&TX_SOCKADDR.into())?;
    socket.set_multicast_if_v4(&TX_ADDRESS)?;

    let socket: UdpSocket = socket.into();

    for i in 1..10 {
        let multicast_addr: Ipv4Addr = format!("239.0.0.{i}").as_str().parse()?;
        let multicast_destination: SocketAddr = (multicast_addr, PORT).into();
        let packet_contents = format!("packet {i}");
        info!(?packet_contents, %multicast_destination, "sender");
        socket.send_to(packet_contents.as_bytes(), multicast_destination)?;
        thread::sleep(Duration::from_millis(200));
    }
    Ok(())
}

fn main() {
    tracing_subscriber::fmt::init();

    info!("multicast-demo is starting");

    let running = Arc::new(AtomicBool::new(true));

    thread::scope(|scope| {
        // Start the receivers
        let running_clone = Arc::clone(&running);
        let receiver1_handle = scope.spawn(move || receiver1(running_clone));
        let running_clone = Arc::clone(&running);
        let receiver2_handle = scope.spawn(move || receiver2(running_clone));

        // Wait for the receivers to initialize
        thread::sleep(Duration::from_millis(500));

        // Run the sender
        let sender_result = sender();

        // Give the receivers time to process the traffic
        thread::sleep(Duration::from_millis(500));

        // Tell the receivers to stop
        running.store(false, Ordering::Relaxed);

        let receiver1_result = receiver1_handle.join();
        let receiver2_result = receiver2_handle.join();

        // Log the results of receivers and sender
        info!(?receiver1_result, ?receiver2_result, ?sender_result);
    });
}
