use serde::Serialize;
use serenity::model::id::{ChannelId};

pub struct Feed {
    pub channel: ChannelId,
    pub host: String,
    pub password: String
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub text: String,
    pub color: String,
    #[serde(rename = "clickEvent")]
    pub click_event: ClickEvent,
    #[serde(rename = "hoverEvent")]
    pub hover_event: HoverEvent
}

#[derive(Debug, Serialize)]
pub struct ClickEvent {
    pub action: String,
    pub value: String
}

#[derive(Debug, Serialize)]
pub struct HoverEvent {
    pub action: String,
    pub contents: Vec<String>
}

#[derive(Debug)]
pub enum TellRaw {
    Message(Message),
    Text(String)
}

impl serde::Serialize for TellRaw {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            TellRaw::Message(message) => serializer.serialize_some(message),
            TellRaw::Text(text) => serializer.serialize_some(text),
        }
    }
}