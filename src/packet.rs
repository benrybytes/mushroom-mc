use crate::{
    byte_handlers::{ByteHandler, RECV_TYPE},
    globals::*,
    handlers::PLAYER_STATES,
    varnums::VARNUM_ERROR,
};
use log::*;
use std::os::fd::{AsRawFd, RawFd};
use tokio::net::TcpStream;

#[derive(PartialEq, Eq)]
pub enum RECV {
    ERROR,
    SUCCESS,
}

pub struct PacketHandler<'a> {
    pub buffer_handler_: &'a mut ByteHandler<'a>,
    pub length: usize,
}

impl<'a> PacketHandler<'a> {
    pub fn new(buffer_handler_: &'a mut ByteHandler<'a>, length: usize) -> Self {
        PacketHandler {
            buffer_handler_,
            length,
        }
    }
    pub async fn handshake(&mut self, state: i32) {
        info! {"state received {}", state};
        if state == STATE_NONE {
            info! {"in state none"};
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
        info! {"state {}", state};
    }

    pub async fn pong(&mut self) {
        self.buffer_handler_.write_byte(9).await;
        self.buffer_handler_.write_byte(0x01).await;
        let u64_value = self.buffer_handler_.read_uint64().await.to_be_bytes();
        self.buffer_handler_.write_n_bytes(&u64_value, 8).await;
    }
    async fn cs_handshake(&mut self) -> RECV {
        info! {"inside cs_handshake"};
        let protocol = self.buffer_handler_.read_varint().await;
        let address = self.buffer_handler_.read_string().await;
        info! {"address: {}", address};
        info! {"protocol: {}", protocol};

        if self.buffer_handler_.recv_count == 0 {
            warn! {"recv_count is zero in cs_handshake"};
            return RECV::ERROR;
        }
        info! {"about to get port"};
        let port = self.buffer_handler_.read_uint16().await;
        info! {"port: {}", port};
        let intent = self.buffer_handler_.read_varint().await;
        info! {"intent: {}", intent};
        if intent == VARNUM_ERROR {
            return RECV::ERROR;
        }
        let mut client_states_ = PLAYER_STATES.lock().await;
        client_states_.insert(self.buffer_handler_.client_fd.as_raw_fd(), intent);
        info! {"client state: {}", client_states_.get(&self.buffer_handler_.client_fd.as_raw_fd()).unwrap()};
        RECV::SUCCESS
    }

    async fn sc_statusResponse(&mut self) -> RECV {
        let header = r###"
    {
        "version":{"name":"1.21.8","protocol":772},
        "description":{"text":""
    "###;
        let footer = r###"
        "}}
    "###;
        let motd = "blahaj's den";
        let string_len = (header.len() + footer.len() + motd.len() - 2) as i32;

        self.buffer_handler_
            .write_varint(1 + string_len + self.buffer_handler_.size_varint(string_len))
            .await;
        self.buffer_handler_.write_byte(0x00).await;

        self.buffer_handler_.write_varint(string_len).await;
        self.buffer_handler_.write_all(header.as_bytes()).await;
        self.buffer_handler_.write_all(motd.as_bytes()).await;
        self.buffer_handler_.write_all(footer.as_bytes()).await;

        RECV::SUCCESS
    }

    // C->S Login Start
    async fn cs_loginStart(&mut self) -> (RECV, Vec<u8>, String) {
        info! {"Received Login Start:\n"};
        self.buffer_handler_.read_string().await;
        let mut name: String = String::with_capacity(16);
        let mut uuid = vec![];
        &self.buffer_handler_.recv_buffer[..16].clone_into(&mut uuid);
        if self.buffer_handler_.recv_count == 0 {
            return (RECV::ERROR, vec![], String::new());
        }
        if let Ok(name_from_buffer) = String::from_utf8(self.buffer_handler_.recv_buffer.to_vec()) {
            name = name_from_buffer;
        } else {
            return (RECV::ERROR, vec![], String::new());
        }
        name.replace_range(15..16, "\0");
        info! {"  Player name: {}\n", name};
        self.buffer_handler_.recv_n_bytes(16, RECV_TYPE::PEEK).await;
        if self.buffer_handler_.recv_count == 0 {
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
        self.buffer_handler_
            .write_varint(1 + 16 + self.buffer_handler_.size_varint(name_length) + name_length + 1)
            .await;
        self.buffer_handler_.write_varint(0x02).await;
        let _ = self.buffer_handler_.write_n_bytes(uuid, 16).await;
        self.buffer_handler_.write_varint(name_length).await;
        let _ = self
            .buffer_handler_
            .write_n_bytes(name.as_bytes(), name_length as usize)
            .await;
        self.buffer_handler_.write_varint(0).await;

        RECV::SUCCESS
    }
}
