use crate::varnums::{VARNUM_ERROR, read_varint};
use log::*;
use tokio::{io, net::TcpStream};

pub async fn handle_client(client_fd: &mut TcpStream) -> Result<(), io::Error> {
    info!(
        "client connected with IP: {}",
        client_fd.local_addr().unwrap()
    );
    // Read packet length
    let length = read_varint(client_fd).await;
    if length == VARNUM_ERROR.try_into().unwrap() {
        // disconnectClient(&clients[client_index], 2);
        // continue;
    }
    // Read packet ID
    let packet_id = read_varint(client_fd).await;
    if packet_id == VARNUM_ERROR.try_into().unwrap() {
        // disconnectClient(&clients[client_index], 3);
        // continue;
    }

    info! {"length: {}", length};
    info! {"packet id {}", packet_id};
    // Handle the client logic here
    // let mut buf = [0; 1024];
    // loop {
    //     socket.readable().await?;
    //     match socket.try_read(&mut buf) {
    //         Ok(0) => break,
    //         Ok(n) => {
    //             info!("read {} bytes", n);
    //         }
    //         Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
    //             continue;
    //         }
    //         Err(e) => {
    //             return Err(e.into());
    //         }
    //     }
    // }

    info!("client connection closed");

    Ok(())
}
