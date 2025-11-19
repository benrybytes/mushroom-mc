static SEGMENT_BITS: i32 = 0x7F;
static CONTINUE_BIT: i32 = 0x80;
pub static VARNUM_ERROR: u32 = 0xFFFFFF;

use crate::byte_handlers::{read_byte, write_byte};
use log::*;
use tokio::net::TcpStream;

pub async fn read_varint(client_fd: &mut TcpStream) -> i32 {
    let mut value: i32 = 0;
    let mut position: i32 = 0;
    let mut current_byte: u8;

    loop {
        current_byte = read_byte(client_fd).await.expect("could not read byte");
        value |= ((current_byte & SEGMENT_BITS as u8) as i32) << position;

        if (current_byte & CONTINUE_BIT as u8) == 0 {
            break;
        }

        position += 7;

        if position >= 32 {
            return VARNUM_ERROR as i32;
        }
    }

    return value;
}

pub fn size_varint(mut value: i32) -> i32 {
    let mut size: i32 = 1;
    while (value & !SEGMENT_BITS) != 0 {
        value >>= 7;
        size += 1;
    }
    return size;
}

pub async fn write_varint(client_fd: &mut TcpStream, mut value: i32) {
    loop {
        if (value & !SEGMENT_BITS) == 0 {
            let _ = write_byte(client_fd, value as u8).await;
            return;
        }

        if let Err(e) = write_byte(client_fd, ((value & SEGMENT_BITS) | CONTINUE_BIT) as u8).await {
            info! {"{:?}", e};
        }

        value >>= 7;
    }
}
