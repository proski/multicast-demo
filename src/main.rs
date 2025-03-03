use anyhow::Result;
use socket2::{Domain, Socket, Type};
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{str, thread};
use tracing::info;

fn receiver1(running: Arc<AtomicBool>) -> Result<()> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
    let local_address: SocketAddr = "127.0.123.1:12345".parse()?;
    socket.set_reuse_address(true)?;
    socket.set_multicast_all_v4(false)?;
    socket.set_read_timeout(Some(Duration::from_millis(50)))?;
    socket.bind(&local_address.into())?;

    let socket: UdpSocket = socket.into();
    let mut buffer = vec![0_u8; 65536];

    while running.load(Ordering::Relaxed) {
        let packet_size = socket.recv(&mut buffer);
        let Ok(packet_size) = packet_size else {
            continue;
        };
        let data = str::from_utf8(&buffer[0..packet_size])?;
        info!(?data, "receiver 1");
    }
    Ok(())
}

fn receiver2(_running: Arc<AtomicBool>) -> Result<()> {
    Ok(())
}

fn sender() -> Result<()> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
    let local_address: SocketAddr = "127.0.123.2:0".parse()?;
    let remote_address: SocketAddr = "127.0.123.1:12345".parse()?;
    socket.bind(&local_address.into())?;
    socket.connect(&remote_address.into())?;
    socket.set_multicast_if_v4(&"127.0.123.2".parse()?)?;

    let socket: UdpSocket = socket.into();

    for i in 0..10 {
        let packet_contents = format!("packet {i}");
        info!(?packet_contents, "sender");
        socket.send(packet_contents.as_bytes())?;
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
