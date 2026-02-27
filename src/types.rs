use serde::Serialize;

#[derive(Serialize)]
pub struct Version {
    pub name: String,
    pub protocol: i32,
}

#[derive(Serialize)]
pub struct Description {
    pub text: String,
}

#[derive(Serialize)]
pub struct Player {
    pub name: String,
    pub id: String,
}

pub struct PlayerData<'a> {
    pub name: String,
    pub uuid: &'a [u8],
}

#[derive(Serialize)]
pub struct Players {
    pub max: i32,
    pub online: i32,
    pub sample: Vec<Player>,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub version: Version,
    pub description: Description,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    pub players: Players,
    #[serde(rename(serialize = "enforcesSecureChat"))]
    pub enforces_secure_chat: bool,
    #[serde(rename(serialize = "previewChats"))]
    pub preview_chats: bool,
}
