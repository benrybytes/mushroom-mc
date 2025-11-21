#![allow(static_mut_refs)]

use super::PacketHandler;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use lazy_static::lazy_static;
use log::*;
use std::io::{Cursor, Error, Write};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::{io, net::TcpStream};

#[allow(non_camel_case_types)]
#[derive(PartialEq, Eq)]
pub enum RECV_TYPE {
    READ,
    PEEK,
}

impl<'a> PacketHandler<'a> {
    async fn recv_buffer_to_big_endian(&mut self) -> u64 {
        // in-memory self.recv_buffer to change
        info! {"recv count: {}", self.recv_count};
        let mut rdr = Cursor::new(&self.recv_buffer);
        ReadBytesExt::read_u64::<BigEndian>(&mut rdr).unwrap()
    }

    pub async fn read_string(&mut self) -> String {
        let length = self.read_varint().await;
        self.recv_n_bytes(length as usize, RECV_TYPE::READ).await;
        self.recv_buffer[self.recv_count] = b'\0';

        String::from_utf8_lossy(&self.recv_buffer[..self.recv_count]).to_string()
    }

    pub async fn read_uint16(&mut self) -> u16 {
        self.recv_n_bytes(2, RECV_TYPE::READ).await;
        ((self.recv_buffer[0] as u16) << 8) | self.recv_buffer[1] as u16
    }

    pub async fn read_uint64(&mut self) -> u16 {
        self.recv_n_bytes(8, RECV_TYPE::READ).await;
        ((self.recv_buffer[0] as u16) << 56)
            | ((self.recv_buffer[1] as u16) << 48)
            | ((self.recv_buffer[2] as u16) << 40)
            | ((self.recv_buffer[3] as u16) << 32)
            | ((self.recv_buffer[4] as u16) << 24)
            | ((self.recv_buffer[5] as u16) << 16)
            | ((self.recv_buffer[6] as u16) << 8)
            | self.recv_buffer[7] as u16
    }

    pub async fn recv_n_bytes(&mut self, n: usize, recv_type: RECV_TYPE) {
        match recv_type {
            RECV_TYPE::READ => {
                self.recv_count = self
                    .client_fd
                    .read_exact(&mut self.recv_buffer[..n])
                    .await
                    .expect("could not read");
            }
            RECV_TYPE::PEEK => {
                self.recv_count = self
                    .client_fd
                    .peek(&mut self.recv_buffer[..n])
                    .await
                    .expect("could not peek");
            }
        };
        self.processed_bytes += self.recv_count;
    }

    pub async fn read_byte(&mut self) -> io::Result<u8> {
        self.recv_count = self.client_fd.read(&mut self.recv_buffer[..1]).await?;
        Ok(self.recv_buffer[0])
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
