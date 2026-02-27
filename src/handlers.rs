use crate::{packets::byte_handlers::*, packets::*};
use lazy_static::lazy_static;
use log::*;
use std::collections::HashMap;
use std::os::fd::RawFd;
use tokio::sync::RwLock;
use tokio::{io, net::TcpStream};

lazy_static! {
    pub static ref PLAYER_STATES: RwLock<HashMap<RawFd, i32>> = RwLock::new(HashMap::new());
}

async fn packet_handle<'a>(packet_handler: &mut PacketHandler<'a>, packet_id: i32) {
    info! {"packet id received: {}", packet_id};

    match packet_id {
        0x00 => packet_handler.handshake().await,
        0x01 => packet_handler.ping().await,
        0x02 => {}
        _ => warn! {"unhandled packet id: 0x{:x}", packet_id},
    }
    let recv_count = packet_handler.recv_count;
    let processed_bytes = packet_handler.processed_bytes;
    // TODO! process client disconnect
    if packet_handler.length != processed_bytes || recv_count == 0 {
        warn! {"did not process full packet"};
    }
    packet_handler.recv_count = 0;
    packet_handler.processed_bytes = 0;
    packet_handler.disconnect_client().await;
}

pub async fn handle_client(client_fd: &mut TcpStream) -> Result<(), io::Error> {
    info!(
        "client connected with IP: {}",
        client_fd.local_addr().unwrap()
    );

    // allow mutable versions of buffer handler with refcell
    let mut packet_handler = PacketHandler::new(client_fd);
    loop {
        // read first 2 bytes

        if let Ok(recieved_bytes) = packet_handler.recv_n_bytes(2, RecvType::Peek).await
            && recieved_bytes >= 2
            && let Ok(length) = packet_handler.read_varint().await
            && let Ok(packet_id) = packet_handler.read_varint().await
        {
            debug! {"length: {}", length};
            debug! {"packet id {}", packet_id};
            let remaining_packet_length = length - (packet_handler.recv_count as i32);
            if remaining_packet_length < 0 {
                warn! {"packet id received, but no data"};
                continue;
            }
            packet_handler.length = remaining_packet_length as usize;
            packet_handle(&mut packet_handler, packet_id).await;
        } else {
            packet_handler.disconnect_client().await;
            continue;
        }
    }
}
