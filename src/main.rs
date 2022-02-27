use lambda_runtime::{Context, Error};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::model::AttributeValue;
use aws_sdk_dynamodb::{Client as DynamodbClient};
use aws_sdk_ssm::{Client as SSMClient, Region};
use serde_json::{json};
use nanoid::nanoid;
use namoral_bot::*;
use reqwest::Client;
use reqwest::header::CONTENT_TYPE;
use rand::seq::SliceRandom;
use rand::Rng;
use std::env;

const TELEGRAM_BASE_URL: &str = "https://api.telegram.org/bot";

#[tokio::main]
async fn main() -> Result<(), Error> {
  let handler = lambda_runtime::handler_fn(handler);
  lambda_runtime::run(handler).await?;
  Ok(())
}

async fn handler(event: Request, context: Context) -> Result<(), Error> {
  if let Err(error) = namoral_bot(event, context).await {
    // this is a very simple bot
    // if telegram do not receive a 200 as a response it will try 
    // to send the update again. This is not necessary in this case
    // so no matter what happens I just log the error
    // and send a empty 200 response
    println!("An error has ocurrer");
    println!("{:?}", error);
  }

  Ok(())
}

async fn namoral_bot(event: Request, _context: Context) -> Result<(), Error> {
  // Get bot_token from secrets manager parameter store
  let token_parameter: String = env::var("token_parameter").unwrap();
  
  let region_provider = RegionProviderChain::default_provider()
    .or_else(Region::new("us-east-1"));
  let shared_config = aws_config::from_env().region(region_provider).load().await;
  let ssm_client = SSMClient::new(&shared_config);

  let response = ssm_client
    .get_parameter()
    .name(token_parameter)
    .with_decryption(true)
    .send()
    .await?;

  let parameter = response.parameter.unwrap();

  let bot_token = parameter.value.unwrap();

  if let Some(value) = event.set_webhook {
    if value {
      let domain = env::var("domain").unwrap();
      let route_path = env::var("route_path").unwrap();

      let set_webhook_url = format!("{}{}/setWebhook", TELEGRAM_BASE_URL, bot_token);

      let message_data = json!({
        "url": format!("{}/{}/", domain, route_path),
      });

      let client = Client::new();
      client.post(set_webhook_url)
          .header(CONTENT_TYPE, "application/json")
          .body(message_data.to_string())
          .send()
          .await?;
    }
    return Ok(());
  }

  let body = event.body.unwrap();
  
  let update: Update = serde_json::from_str(&body).unwrap();
  
  if update.update_id.is_none() {
      // it's not a telegram update
      return Ok(());
  }

  let message: Message = update.message.unwrap();

  let text = message.text.unwrap();
  let send_message_url = format!("{}{}/sendMessage", TELEGRAM_BASE_URL, bot_token);
  let client = reqwest::Client::new();

  let is_private: bool = matches!(message.chat.r#type, ChatType::Private);

  let dynamodb_client = DynamodbClient::new(&shared_config);
  let table_name = env::var("table_name").unwrap();
  let admin_id: u64 = env::var("admin_id").unwrap().parse().unwrap();

  let is_message_from_admin = is_private && admin_id == message.from.id;
  if is_message_from_admin && text.starts_with("/add") {
    // handle /add command that add message to list
    // only the user admin can use commands
    let alphabet: [char; 34] = [
      '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'o', 'p','q', 'r', 's', 't', 'u', 'w', 'x', 'y', 'z'
    ];

    let id_av = AttributeValue::S(nanoid!(10, &alphabet));
    let message_av = AttributeValue::S(text[5..].into());

    dynamodb_client
      .put_item()
      .table_name(table_name)
      .item("id", id_av)
      .item("message", message_av)
      .send()
      .await?;

    let message_data = json!({
      "chat_id": message.chat.id,
      "text": "message added successfully",
    });
  
    client.post(send_message_url)
        .header(CONTENT_TYPE, "application/json")
        .body(message_data.to_string())
        .send()
        .await?;

    return Ok(());
  }

  if is_message_from_admin && text == "/list" {
  // handle /list command that list messages
  // only the user admin can use commands
    let response = dynamodb_client
      .scan()
      .table_name(table_name)
      .send()
      .await?;
    
    let items = response.items.unwrap();

    let message_text = match items.len() {
      0 => String::from("There are no items"),
      _ => {
        let mut temp_text: String = "*Items*".to_string();
        for item in items {
          let item_id = item.get("id").unwrap().as_s().unwrap().as_str();
          let item_message = item.get("message").unwrap().as_s().unwrap().as_str();
          temp_text.push_str(&format!("\n/rm{} {}", item_id, item_message));
        }
        temp_text
      },
    };

    let message_data = json!({
        "chat_id": message.chat.id,
        "text": message_text,
        "parse_mode": "MarkdownV2",
    });
      
    client.post(send_message_url)
      .header(CONTENT_TYPE, "application/json")
      .body(message_data.to_string())
      .send()
      .await?;

    return Ok(());
  }
  if is_message_from_admin && text.starts_with("/rm") {
    // handle /list command that list messages
    // only the user admin can use commands
    let item_id = &text[3..];    

    dynamodb_client
      .delete_item()
      .table_name(table_name)
      .key("id", AttributeValue::S(item_id.into()))
      .send()
      .await?;
      
    let message_data = json!({
      "chat_id": message.chat.id,
      "text": "Item deleted successfully.",
    });

    client.post(send_message_url)
      .header(CONTENT_TYPE, "application/json")
      .body(message_data.to_string())
      .send()
      .await?;

    return Ok(());
  }
  
  let mut rng = rand::thread_rng();

  let should_not_send_message: bool = match message.chat.r#type {
    ChatType::Private => false,
    _ => rng.gen_bool(0.6),
  };

  if should_not_send_message {
    return Ok(());
  }  

  let response = dynamodb_client
    .scan()
    .table_name(table_name)
    .send()
    .await?;

  let items = response.items.unwrap();

  if items.is_empty() {
    // list is empty. No messages to send
    return Ok(());
  }

  let message_to_send = items.choose(&mut rng).unwrap();

  let message_data = json!({
    "chat_id": message.chat.id,
    "text": message_to_send.get("message").unwrap().as_s().unwrap(),
  });

  client.post(send_message_url)
    .header(CONTENT_TYPE, "application/json")
    .body(message_data.to_string())
    .send()
    .await?;

  Ok(())
}
