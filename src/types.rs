use serde::Serialize;

#[derive(Serialize)]
pub struct Version {
    name: String,
    protocol: i32,
}

#[derive(Serialize)]
pub struct Description {
    text: String,
}

#[derive(Serialize)]
pub struct Player {
    name: String,
    id: String,
}

pub struct PlayerData<'a> {
    pub name: String,
    pub uuid: &'a [u8],
}

impl<'a> PlayerData<'a> {
    pub fn new(name: String, uuid: &'a [u8]) -> Self {
        Self { name, uuid }
    }
}

#[derive(Serialize)]
pub struct Players {
    max: i32,
    online: i32,
    sample: Vec<Player>,
}

#[derive(Serialize)]
pub struct StatusResponse {
    version: Version,
    description: Description,
    #[serde(skip_serializing_if = "Option::is_none")]
    favicon: Option<String>,
    players: Players,
    #[serde(rename(serialize = "enforcesSecureChat"))]
    enforces_secure_chat: String,
    #[serde(rename(serialize = "previewChats"))]
    preview_chats: bool,
}
