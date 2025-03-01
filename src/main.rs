use anyhow::Result;
use socket2::{Domain, Socket, Type};
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
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
        info!(?packet_size, "receiver 1");
    }
    Ok(())
}

fn receiver2(_running: Arc<AtomicBool>) -> Result<()> {
    Ok(())
}

fn sender(running: Arc<AtomicBool>) -> Result<()> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
    let local_address: SocketAddr = "127.0.123.1:0".parse()?;
    let remote_address: SocketAddr = "127.0.123.1:12345".parse()?;
    socket.set_reuse_address(true)?;
    socket.set_multicast_all_v4(false)?;
    socket.set_write_timeout(Some(Duration::from_millis(50)))?;
    socket.bind(&local_address.into())?;
    socket.connect(&remote_address.into())?;

    let socket: UdpSocket = socket.into();

    while running.load(Ordering::Relaxed) {
        let packet_size = socket.send(b"data");
        let Ok(packet_size) = packet_size else {
            continue;
        };
        thread::sleep(Duration::from_millis(200));
        info!(?packet_size, "sender");
    }
    Ok(())
}

fn main() {
    tracing_subscriber::fmt::init();

    info!("multicast-demo is starting");

    let running = Arc::new(AtomicBool::new(true));

    thread::scope(|scope| {
        let running_clone = Arc::clone(&running);
        let receiver1_handle = scope.spawn(move || receiver1(running_clone));
        let running_clone = Arc::clone(&running);
        let receiver2_handle = scope.spawn(move || receiver2(running_clone));
        thread::sleep(Duration::from_secs(1));
        let running_clone = Arc::clone(&running);
        let sender_handle = scope.spawn(move || sender(running_clone));

        thread::sleep(Duration::from_secs(1));
        running.store(false, Ordering::Relaxed);

        let receiver1_result = receiver1_handle.join();
        let receiver2_result = receiver2_handle.join();
        let sender_result = sender_handle.join();
        info!(?receiver1_result, ?receiver2_result, ?sender_result);
    });
}
