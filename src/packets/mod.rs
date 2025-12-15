use log::*;
use std::os::fd::AsRawFd;
use tokio::net::TcpStream;

pub mod byte_handlers;
pub mod varnums;
use crate::globals::*;
use crate::handlers::PLAYER_STATES;

static DEFAULT_SIZE: usize = 1024;

#[derive(PartialEq, Eq)]
pub enum RecvStatus {
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
            if self.cs_handshake().await == RecvStatus::ERROR {
                warn! {"cs_handshake unsuccessful"};
                return;
            }
        } else if state == STATE_STATUS {
            info! { "sending status" };
            if self.sc_status_response().await == RecvStatus::ERROR {
                warn! {"sc_statusResponse unsuccessful"};
                return;
            }
        }

        if state == STATE_LOGIN {
            let (name, uuid) = self.cs_login_start().await.expect("could not get");

            let _ = self.sc_login_success(name, &uuid).await.unwrap();
        } else if state == STATE_CONFIGURATION {
            //     if cs_clientInformation(client_fd) || sc_knownPacks(client_fd) || sc_registries(client_fd) {
            //         return;
            self.sc_send_plugin_message("minecraft:brand", BRAND).await;
        }

        if state == STATE_LOGIN {}
    }

    /// sends ping to client
    pub async fn ping(&mut self) {
        let state = self.client_state;
        // No need for a packet handler, just echo back the long verbatim
        if state == STATE_STATUS
            && let Ok(read_value) = self.read_u64().await
            && let Ok(_) = self.write_byte(9).await
            && let Ok(_) = self.write_byte(0x01).await
            && let Ok(_) = self.write_n_bytes(&read_value.to_le_bytes(), 8).await
        {
            self.recv_count = 0;
            debug!("successfully pinged client");
        }
    }

    pub async fn cs_handshake(&mut self) -> RecvStatus {
        debug!("RECV COUNT: {}", self.recv_count);
        let mut client_states_ = PLAYER_STATES.write().await;
        if let None = client_states_.get(&self.client_fd.as_raw_fd())
            && let Ok(protocol) = self.read_varint().await
            && let Ok(address) = self.read_string().await
        {
            info!("address: {address}");
            if let Ok(port) = self.read_u16().await
                && let Ok(intent) = self.read_varint().await
            {
                debug! {"port: {}", port};
                debug! {"intent: {}", intent};
                debug! {"address: {}", address};
                debug! {"protocol: {}", protocol};
                client_states_.insert(self.client_fd.as_raw_fd(), intent);
                self.client_state = intent;
            }
            RecvStatus::SUCCESS
        } else {
            error!("user disconnected or invalid read for cs_handshake");
            RecvStatus::ERROR
        }
    }

    pub async fn sc_send_plugin_message(&mut self, channel: &str, data: &str) -> RecvStatus {
        let channel_len = channel.len() as i32;
        let data_len = data.len() as i32;

        let _ = self.write_varint(
            1 + Self::size_varint(channel_len) as i32
                + channel_len
                + Self::size_varint(data_len) as i32
                + data_len,
        );
        let _ = self.write_byte(0x01);

        let _ = self.write_varint(channel_len);
        let _ = self.write_n_bytes(channel.as_bytes(), channel_len as usize);
        let _ = self.write_varint(data_len);

        let _ = self.write_n_bytes(data.as_bytes(), data_len as usize);

        RecvStatus::SUCCESS
    }
    async fn sc_status_response(&mut self) -> RecvStatus {
        // 1. Prepare the JSON data (DO NOT REVERSE)
        let response_json = br###"
        {
            "version": {
                "name": "1.21.11",
                "protocol": 774
            },
            "description": {
                "text": "{BRAND}"
            },
            "favicon": "../../world/icon.png",
            "players": {
                "max": 20,
                "online": 0
            },
            "enforcesSecureChat": false
        }
        "###;

        // The data for the String field (Length VarInt + JSON Data)
        let string_data_len = response_json.len() as i32;
        let string_field_size = Self::size_varint(string_data_len) + string_data_len as u32;

        // The total length of the Packet ID (0x00) + String Field
        // Packet ID is 0x00, which takes 1 byte as a VarInt.
        let packet_data_length = 1 + string_field_size;

        // 2. Write the overall Packet Length prefix (as a VarInt)
        if let Err(e) = self.write_varint(packet_data_length as i32).await {
            error! {"could not write packet length {:?}", e};
            return RecvStatus::ERROR;
        }

        // 3. Write the Packet ID (0x00 as a VarInt)
        if let Err(e) = self.write_varint(0x00).await {
            error! {"could not write packet ID {:?}", e};
            return RecvStatus::ERROR;
        }

        // 4. Write the String (which handles its own length prefix)
        if let Err(e) = self.write_utf8_string(response_json).await {
            error! {"could not write status JSON string {:?}", e};
            return RecvStatus::ERROR;
        }

        info! {"SC Status Response sent successfully"};
        RecvStatus::SUCCESS
    }
    // C->S Login Start
    async fn cs_login_start(&mut self) -> Result<(String, [u8; 16]), &str> {
        info! {"Received Login Start:\n"};
        // read username
        if let Ok(name) = self.read_string().await
            && let Ok(uuid) = self.read_uuid().await
        {
            info! {"name: {name}"};
            info! {"player name: {}", name};
            info! {"player UUID: {:?}", uuid};
            Ok((name, uuid))
        } else {
            Err("unknown player?")
        }
    }

    // S -> C
    async fn sc_login_success(&mut self, name: String, uuid: &[u8]) -> Result<(), &str> {
        debug!("Sending Login Success...\n\n");
        // sc_login_success
        if let Ok(_) = self
            .write_varint(
                (1 + 16 + Self::size_varint(name.len() as i32) + name.len() as u32 + 1) as i32,
            )
            .await
            && let Ok(_) = self.write_varint(0x02).await
            && let Ok(_) = self.write_n_bytes(&uuid, 16).await
            && let Ok(_) = self.write_varint(name.len() as i32).await
            && let Ok(_) = self
                .write_n_bytes(name.as_bytes(), name.len() as usize)
                .await
            && let Ok(_) = self.write_varint(0).await
        {
            Ok(())
        } else {
            Err("error logging in successfully")
        }
    }
}
