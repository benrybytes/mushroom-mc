use crate::packet::PacketHandler;
use crate::{
    byte_handlers::{ByteHandler, RECV_TYPE},
    globals::*,
    varnums::VARNUM_ERROR,
};

use lazy_static::lazy_static;
use log::*;
use std::collections::HashMap;
use std::os::fd::{AsRawFd, RawFd};
use tokio::{io, net::TcpStream, sync::Mutex};

lazy_static! {
    pub static ref PLAYER_STATES: Mutex<HashMap<RawFd, i32>> = Mutex::new(HashMap::new());
}

async fn packet_handle<'a>(packet_handler: &mut PacketHandler<'a>, packet_id: i32) {
    info! {"packet id received: {}", packet_id};
    let player_states_ = PLAYER_STATES.lock().await;
    let mut player_state = STATE_NONE;
    if let Some(state) = player_states_.get(&packet_handler.buffer_handler_.client_fd.as_raw_fd()) {
        player_state = *state;
    }
    info! {"player state: {}", player_state};

    match packet_id {
        0x00 => packet_handler.handshake(player_state).await,
        0x01 => packet_handler.pong().await,
        _ => warn! {"0x{:x}", packet_id},
    }
    // TODO! process client disconnect
    if packet_handler.length != packet_handler.buffer_handler_.processed_bytes {
        warn! {"did not process full packet"};
    }
}

pub async fn handle_client(client_fd: &mut TcpStream) -> Result<(), io::Error> {
    info!(
        "client connected with IP: {}",
        client_fd.local_addr().unwrap()
    );

    let mut buffer_handler_ = ByteHandler::new(client_fd);

    buffer_handler_.recv_n_bytes(2, RECV_TYPE::PEEK).await;
    if buffer_handler_.recv_count < 2 {
        return Err(io::Error::other("not enough bytes to read"));
    }

    // Read packet length
    let length = buffer_handler_.read_varint().await;
    if length == VARNUM_ERROR {
        // disconnectClient(&clients[client_index], 2);
        // continue;
    }
    // Read packet ID
    let packet_id = buffer_handler_.read_varint().await;
    if packet_id == VARNUM_ERROR {
        // disconnectClient(&clients[client_index], 3);
        // continue;
    }
    info! {"length: {}", length};
    info! {"packet id {}", packet_id};
    let remaining_packet_length = length as usize - (buffer_handler_.recv_count);
    let mut packet_handler_ =
        PacketHandler::new(&mut buffer_handler_, remaining_packet_length as usize);
    packet_handle(&mut packet_handler_, packet_id).await;

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
