use log::*;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::os::fd::{AsRawFd, RawFd};
use std::rc::Rc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

pub mod byte_handlers;
pub mod varnums;
use crate::globals::*;
use crate::handlers::PLAYER_STATES;

use crate::packets::varnums::VARNUM_ERROR;
use byte_handlers::RECV_TYPE;
use varnums::*;

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
            info! { "sending status" };
            if self.sc_statusResponse().await == RECV::ERROR {
                warn! {"sc_statusResponse unsuccessful"};
                return;
            }
        }

        if state == STATE_LOGIN {
            info! {"player logged in"};
            let (mut recv_status, mut uuid, mut name) = self.cs_loginStart().await;
            info! {"name: {}", name};
            info! {"uuid: {:?}", uuid};
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
        debug! {"client state: {}", client_states_.get(&self.client_fd.as_raw_fd()).unwrap()};
        RECV::SUCCESS
    }

    // Assuming helper functions exist:
    // async fn write_varint(&mut self, value: i32) -> io::Result<()>;
    // fn size_varint(value: i32) -> usize;

    async fn sc_statusResponse(&mut self) -> RECV {
        // 1. Prepare the JSON data (DO NOT REVERSE)
        let response_json = br###"
        {
            "version": {
                "name": "1.21.10",
                "protocol": 772
            },
            "description": {
                "text": "blahaj world"
            },
            "players": {
                "max": 20,
                "online": 0
            }
        }
        "###;

        // The data for the String field (Length VarInt + JSON Data)
        let string_data_len = response_json.len() as i32;
        let string_field_size = self.size_varint(string_data_len) + string_data_len as u32;

        // The total length of the Packet ID (0x00) + String Field
        // Packet ID is 0x00, which takes 1 byte as a VarInt.
        let packet_data_length = 1 + string_field_size;

        // 2. Write the overall Packet Length prefix (as a VarInt)
        if let Err(e) = self.write_varint(packet_data_length as i32).await {
            error! {"could not write packet length {:?}", e};
            return RECV::ERROR;
        }

        // 3. Write the Packet ID (0x00 as a VarInt)
        if let Err(e) = self.write_varint(0x00).await {
            error! {"could not write packet ID {:?}", e};
            return RECV::ERROR;
        }

        // 4. Write the String (which handles its own length prefix)
        if let Err(e) = self.write_utf8_string(response_json).await {
            error! {"could not write status JSON string {:?}", e};
            return RECV::ERROR;
        }

        info! {"SC Status Response sent successfully"};
        RECV::SUCCESS
    }
    // C->S Login Start
    async fn cs_loginStart(&mut self) -> (RECV, Vec<u8>, String) {
        info! {"Received Login Start:\n"};

        // read username
        self.read_string().await;
        let mut name: String = String::with_capacity(16);
        let mut uuid = vec![];
        if let Ok(name_from_buffer) = String::from_utf8(self.recv_buffer.to_vec()) {
            name = name_from_buffer;
        } else {
            return (RECV::ERROR, vec![], String::new());
        }
        name.replace_range(15..16, "\0");
        info! {"  Player name: {}\n", name};

        // read UUID
        self.recv_n_bytes(16, RECV_TYPE::READ).await;
        &self.recv_buffer[..16].clone_into(&mut uuid);
        if self.recv_count == 0 {
            return (RECV::ERROR, vec![], String::new());
        }
        info! {"Player UUID: "};
        for i in 0..16 {
            info! {"{:x}\0", uuid[i]};
        }
        info!("\n");

        (RECV::SUCCESS, uuid, name)
    }

    // S->C Login Success
    async fn sc_loginSuccess(&mut self, uuid: &[u8], name: &String) -> RECV {
        info!("Sending Login Success...\n\n");

        let name_length = name.len() as i32;
        let value = 1 + 16 + self.size_varint(name_length) + name_length as u32 + 1;
        self.write_varint(value as i32).await;
        self.write_varint(0x02).await;
        self.write_n_bytes(uuid, 16).await;
        self.write_varint(name_length).await;
        self.write_n_bytes(name.as_bytes(), name_length as usize)
            .await;
        self.write_varint(0).await;

        RECV::SUCCESS
    }
}
