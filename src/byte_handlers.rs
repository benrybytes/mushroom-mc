#![allow(static_mut_refs)]

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use lazy_static::lazy_static;
use log::*;
use std::cell::RefCell;
use std::io::{Cursor, Error, Write};
use std::rc::Rc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::{io, net::TcpStream};

#[derive(PartialEq, Eq)]
pub enum RECV_TYPE {
    READ,
    PEEK,
}
static DEFAULT_SIZE: usize = 1024;

pub struct ByteHandler<'a> {
    pub recv_buffer: [u8; DEFAULT_SIZE],
    pub recv_count: usize,
    pub client_fd: &'a mut TcpStream,
    pub processed_bytes: usize,
}

impl<'a> ByteHandler<'a> {
    pub fn new(client_fd: &'a mut TcpStream) -> Self {
        ByteHandler {
            recv_buffer: [0_u8; DEFAULT_SIZE],
            recv_count: 0,
            client_fd,
            processed_bytes: 0,
        }
    }

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
        info! {"recv count in string: {}", self.recv_count};
        info! {"recv buffer: {:?}", self.recv_buffer};

        String::from_utf8_lossy(&self.recv_buffer[..self.recv_count]).to_string()
    }

    pub async fn read_uint16(&mut self) -> u16 {
        self.recv_n_bytes(2, RECV_TYPE::READ).await;
        info! {"utf16: {:?}", &self.recv_buffer[..2]};
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
        self.recv_count = self.client_fd.read(&mut self.recv_buffer[..1]).await?; // Read exactly one byte
        //
        Ok(self.recv_buffer[0]) // Return the byte
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
