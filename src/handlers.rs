use crate::{globals::*, packets::byte_handlers::*, packets::varnums::*, packets::*};
use lazy_static::lazy_static;
use log::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::os::fd::{AsRawFd, RawFd};
use std::rc::Rc;
use tokio::sync::RwLock;
use tokio::{io, net::TcpStream, sync::Mutex};

lazy_static! {
pub static ref PLAYER_STATES: RwLock<HashMap<RawFd, i32>> =
    RwLock::new(HashMap::new());
//     pub static ref PLAYER_STATES: Mutex<HashMap<RawFd, i32>> = Mutex::new(HashMap::new());
}

fn get_player_state(client_fd: &TcpStream) {}

async fn packet_handle<'a>(packet_handler: &mut PacketHandler<'a>, packet_id: i32) {
    info! {"packet id received: {}", packet_id};

    match packet_id {
        0x00 => packet_handler.handshake().await,
        0x01 => packet_handler.ping().await,
        _ => warn! {"0x{:x}", packet_id},
    }
    let recv_count = packet_handler.recv_count;
    let processed_bytes = packet_handler.processed_bytes;
    // TODO! process client disconnect
    if packet_handler.length != processed_bytes || recv_count == 0 {
        warn! {"did not process full packet"};
    }
}

pub async fn handle_client(client_fd: &mut TcpStream) -> Result<(), io::Error> {
    info!(
        "client connected with IP: {}",
        client_fd.local_addr().unwrap()
    );

    // allow mutable versions of buffer handler with refcell
    let mut packet_handler = PacketHandler::new(client_fd);
    loop {
        packet_handler.recv_n_bytes(2, RECV_TYPE::PEEK).await;
        if packet_handler.recv_count < 2 {
            continue;
        }

        // Read packet length
        let length = packet_handler.read_varint().await;
        if length == VARNUM_ERROR {
            // disconnectClient(&clients[client_index], 2);
            continue;
        }
        // Read packet ID
        let packet_id = packet_handler.read_varint().await;
        if packet_id == VARNUM_ERROR {
            // disconnectClient(&clients[client_index], 3);
            continue;
        }
        info! {"length: {}", length};
        info! {"packet id {}", packet_id};
        let remaining_packet_length = length - (packet_handler.recv_count as i32);
        if remaining_packet_length < 0 {
            warn! {"packet id received, but no data"};
            continue;
        }
        packet_handler.length = remaining_packet_length as usize;
        packet_handle(&mut packet_handler, packet_id).await;
        packet_handler.recv_count = 0;
    }
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

fn _disconnect_client() {
    todo! {}
}
