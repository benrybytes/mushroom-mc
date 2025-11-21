use log::*;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::os::fd::{AsRawFd, RawFd};
use std::rc::Rc;
use tokio::net::TcpStream;

pub mod byte_handlers;
pub mod varnums;
use crate::globals::*;
use crate::handlers::PLAYER_STATES;

use crate::packets::varnums::VARNUM_ERROR;
use byte_handlers::RECV_TYPE;

static DEFAULT_SIZE: usize = 1024;

#[derive(PartialEq, Eq)]
pub enum RECV {
    ERROR,
    SUCCESS,
}

pub struct PacketHandler<'a> {
    recv_buffer: [u8; DEFAULT_SIZE],
    pub recv_count: usize,
    pub client_fd: &'a mut TcpStream,
    client_state: i32,
    pub processed_bytes: usize,
    pub length: usize,
}

impl<'a> PacketHandler<'a> {
    pub fn new(client_fd: &'a mut TcpStream) -> Self {
        PacketHandler {
            client_fd,
            client_state: 0,
            length: 0,
            recv_buffer: [0_u8; DEFAULT_SIZE],
            recv_count: 0,
            processed_bytes: 0,
        }
    }
    pub async fn handshake(&mut self) {
        let state = self.client_state;
        debug! {"state received {}", state};
        if state == STATE_NONE {
            debug! {"in state none"};
            if self.cs_handshake().await == RECV::ERROR {
                warn! {"cs_handshake unsuccessful"};
                return;
            }
        } else if state == STATE_STATUS {
            if self.sc_statusResponse().await == RECV::ERROR {
                warn! {"sc_statusResponse unsuccessful"};
                return;
            }
        }

        if state == STATE_LOGIN {
            let (mut recv_status, mut uuid, mut name) = self.cs_loginStart().await;
            if recv_status == RECV::ERROR {
                return;
            }
            // if reservePlayerData(client_fd, uuid, name) {
            //     recv_count = 0;
            //     return;
            // }
            recv_status = self.sc_loginSuccess(&mut uuid, &mut name).await;
            if recv_status == RECV::ERROR {
                return;
            }
            // } else if (state == STATE_CONFIGURATION) {
            //     if cs_clientInformation(client_fd) || sc_knownPacks(client_fd) || sc_registries(client_fd) {
            //         return;
            //     }
        }
        debug! {"state {}", state};
    }

    pub async fn ping(&mut self) {
        let state = self.client_state;
        if state == STATE_STATUS {
            // No need for a packet handler, just echo back the long verbatim
            self.write_byte(9).await;
            self.write_byte(0x01).await;
            let read_value = self.read_uint64().await;
            self.write_n_bytes(&read_value.to_le_bytes(), 8).await;
            self.recv_count = 0;
            debug! {"read_value {}", read_value};
        }
    }

    async fn cs_handshake(&mut self) -> RECV {
        let protocol = self.read_varint().await;
        let address = self.read_string().await;
        debug! {"address: {}", address};
        debug! {"protocol: {}", protocol};

        if self.recv_count == 0 {
            warn! {"recv_count is zero in cs_handshake"};
            return RECV::ERROR;
        }
        let port = self.read_uint16().await;
        debug! {"port: {}", port};
        let intent = self.read_varint().await;
        debug! {"intent: {}", intent};
        if intent == VARNUM_ERROR {
            warn! {"intent not found"};
            return RECV::ERROR;
        }
        debug! {"before client states"};
        let mut client_states_ = PLAYER_STATES.write().await;
        client_states_.insert(self.client_fd.as_raw_fd(), intent);
        self.client_state = intent;

        debug! {"{:?}", client_states_};
        debug! {"client state: {}", client_states_.get(&self.client_fd.as_raw_fd()).unwrap()};
        RECV::SUCCESS
    }

    async fn sc_statusResponse(&mut self) -> RECV {
        let header = r###"
    {
        "version":{"name":"1.21.8","protocol":773},
        "description":{"text":""
    "###;
        let footer = r###"
        "}}
    "###;
        let motd = "blahaj's adventure";
        let string_len = (header.len() + footer.len() + motd.len() - 2) as i32;

        self.write_varint(1 + string_len + self.size_varint(string_len))
            .await;
        self.write_byte(0x00).await;

        self.write_varint(string_len).await;
        self.write_all(header.as_bytes()).await;
        self.write_all(motd.as_bytes()).await;
        self.write_all(footer.as_bytes()).await;

        RECV::SUCCESS
    }

    // C->S Login Start
    async fn cs_loginStart(&mut self) -> (RECV, Vec<u8>, String) {
        info! {"Received Login Start:\n"};
        self.read_string().await;
        let mut name: String = String::with_capacity(16);
        let mut uuid = vec![];
        &self.recv_buffer[..16].clone_into(&mut uuid);
        if self.recv_count == 0 {
            return (RECV::ERROR, vec![], String::new());
        }
        if let Ok(name_from_buffer) = String::from_utf8(self.recv_buffer.to_vec()) {
            name = name_from_buffer;
        } else {
            return (RECV::ERROR, vec![], String::new());
        }
        name.replace_range(15..16, "\0");
        info! {"  Player name: {}\n", name};
        self.recv_n_bytes(16, RECV_TYPE::PEEK).await;
        if self.recv_count == 0 {
            return (RECV::ERROR, vec![], String::new());
        }
        info! {"Player UUID: "};
        for i in 0..16 {
            info! {"{:x}", uuid[i]};
        }
        info!("\n\n");

        (RECV::SUCCESS, uuid, name)
    }

    // S->C Login Success
    async fn sc_loginSuccess(&mut self, uuid: &[u8], name: &String) -> RECV {
        info!("Sending Login Success...\n\n");

        let name_length: i32 = name.len() as i32;
        self.write_varint(1 + 16 + self.size_varint(name_length) + name_length + 1)
            .await;
        self.write_varint(0x02).await;
        let _ = self.write_n_bytes(uuid, 16).await;
        self.write_varint(name_length).await;
        let _ = self
            .write_n_bytes(name.as_bytes(), name_length as usize)
            .await;
        self.write_varint(0).await;

        RECV::SUCCESS
    }
}
