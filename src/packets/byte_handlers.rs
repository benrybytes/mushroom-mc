#![allow(static_mut_refs)]

use super::PacketHandler;
use crate::globals::{INVALID_READ, VARNUM_ERROR};
use log::*;
use paste::paste;
use std::{io::Error, mem};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

#[allow(non_camel_case_types)]
#[derive(PartialEq, Eq)]
pub enum RecvType {
    Read,
    Peek,
}

macro_rules! read_bytes {
    ($recv_prefixes:expr, $recv_type:expr, [$($type:ty),*]) => {
        paste! {
            #[allow(unused)]
            impl<'a> PacketHandler<'a> {
                $(
                    pub async fn [<$recv_prefixes _ $type>](&mut self) -> Result<$type, i32> {
                        const N_BYTES: usize = mem::size_of::<$type>();
        self.recv_count = N_BYTES;
                        info!("n bytes: {}", N_BYTES);
                        if let Err(e) = self.recv_n_bytes(N_BYTES, $recv_type).await {
        error!("ERROR RECEIVING BYTES :c {:?}", e);
                        return Err(VARNUM_ERROR);
        }
        info!("fjoifjwe: {:?}", &self.recv_buffer[0..16]);
        let mut buffer_temp = [0u8; N_BYTES];
        buffer_temp.clone_from_slice(&self.recv_buffer[..N_BYTES]);
                        let mut bytes_read: $type = $type::from_le_bytes(buffer_temp);

        info!("BYTES: {}", bytes_read);
                        Ok(bytes_read)
                    }
                )*
            }
        }
    };
}

read_bytes!("read", RecvType::Read, [u8, u16, u32, u64]);
read_bytes!("peek", RecvType::Peek, [u8, u16, u32, u64]);

impl<'a> PacketHandler<'a> {
    pub async fn read_uuid(&mut self) -> Result<[u8; 16], Error> {
        self.recv_n_bytes(16, RecvType::Read).await?;
        let mut uuid = [0u8; 16];
        uuid.clone_from_slice(&self.recv_buffer[..16]);
        Ok(uuid)
    }
    pub async fn read_string(&mut self) -> Result<String, i32> {
        if let Ok(length) = self.read_varint().await {
            debug!("LENGTH: {length}");
            if let Ok(_) = self.recv_n_bytes(length as usize, RecvType::Read).await {
                debug!("got string");
                self.recv_buffer[self.recv_count] = b'\0';
                self.recv_count += 1;
                Ok(String::from_utf8_lossy(&self.recv_buffer[..self.recv_count]).to_string())
            } else {
                Err(INVALID_READ)
            }
        } else {
            error!("invalid string reaed");
            Err(INVALID_READ)
        }
    }

    /// this method reads n bytes from client
    pub async fn recv_n_bytes(&mut self, n: usize, recv_type: RecvType) -> Result<usize, Error> {
        match recv_type {
            RecvType::Read => {
                self.recv_count = self
                    .client_fd
                    .read_exact(&mut self.recv_buffer[..n])
                    .await?;
                self.processed_bytes += self.recv_count;
            }
            RecvType::Peek => {
                self.recv_count = self.client_fd.peek(&mut self.recv_buffer[..n]).await?;
            }
        };
        Ok(self.recv_count)
    }

    pub async fn write_utf8_string(&mut self, data: &[u8]) -> io::Result<()> {
        // 1. Get the length of the UTF-8 bytes
        let data_length = data.len() as i32;

        // 2. Write the length as a VarInt (The ONLY length prefix needed for the string)
        self.write_varint(data_length).await?;

        // 3. Write the actual UTF-8 encoded string data
        // Assuming self.write_all(data) is equivalent to writing all bytes in the slice.
        self.write_all(data).await?;

        Ok(())
    }
    pub async fn write_byte(&mut self, value: u8) -> io::Result<()> {
        self.client_fd.write_u8(value).await?;
        self.client_fd.flush().await?;
        Ok(())
    }

    pub async fn write_all(&mut self, value: &[u8]) -> io::Result<()> {
        self.client_fd.write_all(value).await?;
        self.client_fd.flush().await?;
        Ok(())
    }

    pub async fn write_n_bytes(&mut self, value: &[u8], size: usize) -> io::Result<()> {
        self.client_fd.write_all(&value[..size]).await?;
        self.client_fd.flush().await?;
        Ok(())
    }
}
