#![allow(dead_code)]
#![allow(unused_variables)]
use serde::{Serialize, Deserialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatType {
  Private,
  Group,
  Supergroup,
  Channel,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chat {
  first_name: Option<String>,
  title: Option<String>,
  pub id: i64,
  pub r#type: ChatType,
  username: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct From {
  first_name: String,
  pub id: u64,
  is_bot: bool,
  language_code: String,
  username: String,
}

#[derive(Debug, Serialize,Deserialize)]
pub struct Message {
  pub chat: Chat,
  date: u64,
  pub from: From,
  message_id: u64,
  pub text: Option<String>,
}

#[derive(Debug, Serialize,Deserialize)]
pub struct Update {
  pub message: Option<Message>,
  pub update_id: Option<u64>,
}

#[derive(Debug, Serialize,Deserialize)]
pub struct Request {
  pub body: Option<String>,
  pub set_webhook: Option<bool>,
}