use log::*;
use std::os::fd::AsRawFd;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

pub mod byte_handlers;
pub mod varnums;
use crate::globals::*;
use crate::handlers::PLAYER_STATES;
use crate::types::*;

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

    pub async fn disconnect_client(&mut self) {
        if let Err(e) = self.client_fd.flush().await
            && let None = PLAYER_STATES
                .write()
                .await
                .remove(&self.client_fd.as_raw_fd())
        {
            error!("invalid cleaning up client \nreason{e}");
        }
    }

    pub async fn handshake(&mut self) {
        let state = self.client_state;
        debug! {"state received {}", state};
        if state == STATE_NONE
            && let Err(error_message) = self.cs_handshake().await
        {
            error! {"{error_message}"};
            self.disconnect_client().await;
            return;
        } else if state == STATE_STATUS
            && let Err(error_message) = self.sc_status_response().await
        {
            error! {"{error_message}"};
            self.disconnect_client().await;
        }

        if state == STATE_LOGIN
            && let Ok((name, uuid)) = self.cs_login_start().await
            && let Ok(()) = self.sc_login_success(name, &uuid).await
        {
        } else if state == STATE_CONFIGURATION &&
            let Ok(_) = self.cs_client_information().await &&
            //     if cs_clientInformation(client_fd) || sc_knownPacks(client_fd) || sc_registries(client_fd) {
            //         return;
            let Err(error_message) = self.sc_send_plugin_message("minecraft:brand", BRAND).await
        {
            error! {"{error_message}"};
        }
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

    pub async fn cs_handshake(&mut self) -> Result<(), &str> {
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
            Ok(())
        } else {
            Err("user disconnected or invalid read for cs_handshake")
        }
    }

    pub async fn sc_send_plugin_message(&mut self, channel: &str, data: &str) -> Result<(), &str> {
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

        Ok(())
    }

    async fn sc_status_response(&mut self) -> Result<(), &str> {
        // 1. Prepare the JSON data (DO NOT REVERSE)
        let response_json = StatusResponse {
            version: Version {
                name: String::from("1.21.11"),
                protocol: 774,
            },
            description: Description {
                text: String::from("blahaj minecraft server"),
            },
            favicon: Some(String::from("null data for now")),
            players: Players {
                max: 20,
                online: 0,
                sample: vec![],
            },
            enforces_secure_chat: false,
            preview_chats: true,
        };

        if let Ok(response_json) = serde_json::to_string(&response_json) {
            // The data for the String field (Length VarInt + JSON Data)
            let string_data_len = response_json.len() as i32;
            let string_field_size = Self::size_varint(string_data_len) + string_data_len as u32;

            // The total length of the Packet ID (0x00) + String Field
            // Packet ID is 0x00, which takes 1 byte as a VarInt.
            let packet_data_length = 1 + string_field_size;

            // 2. Write the overall Packet Length prefix (as a VarInt)
            if let Err(_) = self.write_varint(packet_data_length as i32).await &&
            // 3. Write the Packet ID (0x00 as a VarInt)
            let Err(_) = self.write_varint(0x00).await &&

            // 4. Write the String (which handles its own length prefix)

            let Err(_) = self.write_utf8_string(&response_json.into_bytes()).await
            {
                error!("could not send status response")
            }

            info! {"SC Status Response sent successfully"};
            Ok(())
        } else {
            Err("could not check successfully logging in")
        }
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
            // send length of length, uuid, and name together
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

    async fn sc_known_packs(&mut self) -> Result<(), &str> {}

    async fn cs_client_information(&mut self) -> Result<(), &str> {
        if let Ok(locale) = self.read_string().await
            && let Ok(view_distance) = self.read_u8().await
            && let Ok(chat_mode) = self.read_varint().await
            && let Ok(chat_color) = self.read_u8().await

        // capes and other stuff about skin
            && let Ok(displayed_skin_parts) = self.read_u8().await
            && let Ok(main_hand) = self.read_varint().await
            && let Ok(text_filter) = self.read_u8().await
            && let Ok(server_listing) = self.read_u8().await
            && let Ok(particle_status) = self.read_varint().await
        {
            info!(
                "{locale}
                {view_distance}
                {chat_mode}
                {particle_status}
            "
            );
            Ok(())
        } else {
            Err("issue getting client information")
        }
    }
}
