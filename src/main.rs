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

async fn handler(event: Request, _context: Context) -> Result<(), Error> {
  let token_parameter: String;
  match env::var("token_parameter") {
    Ok(value) => token_parameter = value,
    _ => {
      println!("Unable to get token name from env");
      return Ok(());
    }
  }
  
  let region_provider = RegionProviderChain::default_provider()
    .or_else(Region::new("us-east-1"));
  let shared_config = aws_config::from_env().region(region_provider).load().await;
  let ssm_client = SSMClient::new(&shared_config);

  let response = ssm_client
    .get_parameter()
    .name(token_parameter)
    .with_decryption(true)
    .send()
    .await;

  if let Err(_err) = response {
    println!("Error trying to get bot token");
    return Ok(());
  }

  let response = response.unwrap();

  let parameter = response.parameter;

  if parameter.is_none() {
    println!("No parameter");
    return Ok(());
  }

  let parameter = parameter.unwrap();

  if parameter.value.is_none() {
    println!("Parameter has no value");
    return Ok(());
  }

  let bot_token = parameter.value.unwrap();

  if let Some(value) = event.set_webhook {
    if value {
      let domain = env::var("domain").unwrap();
      let route_path = env::var("route_path").unwrap();

      let set_webhook_url = format!("{}{}/setWebhook", TELEGRAM_BASE_URL, bot_token);

      let message_data = json!({
        "url": format!("{}/{}/", domain, route_path),
      });

      println!("telegram url: {:?} || webhook_url: {:?}", set_webhook_url, message_data);

      let client = Client::new();
      let res = client.post(set_webhook_url)
          .header(CONTENT_TYPE, "application/json")
          .body(message_data.to_string())
          .send()
          .await;

      if let Err(error) = res {
        println!("Error trying to set webhook");
        println!("{:?}", error);
      }
    }
    return Ok(());
  }

  if event.body.is_none() {
    println!("Request has no body");
    return Ok(());
  }

  let body = event.body.unwrap();
  
  let update: Update = serde_json::from_str(&body).unwrap();
  
  println!("{:?}", update);

  if update.update_id.is_none() {
      println!("Event it's not an telegram update");
      return Ok(());
  }
  if update.message.is_none() {
      println!("Update it's not a message");
      return Ok(());
  }

  let message: Message = update.message.unwrap();

  if message.text.is_none() {
      println!("Update it's not a text message");
      return Ok(());
  }

  let text = message.text.unwrap();
  let send_message_url = format!("{}{}/sendMessage", TELEGRAM_BASE_URL, bot_token);
  let client = reqwest::Client::new();

  let is_private: bool = matches!(message.chat.r#type, ChatType::Private);

  let dynamodb_client = DynamodbClient::new(&shared_config);
  let table_name = env::var("table_name").unwrap();
  let admin_id: u64 = env::var("admin_id").unwrap().parse().unwrap();

  let is_message_from_admin = is_private && admin_id == message.from.id;

  if is_message_from_admin && text.starts_with("/add") {
    let alphabet: [char; 34] = [
      '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'o', 'p','q', 'r', 's', 't', 'u', 'w', 'x', 'y', 'z'
    ];

    let id_av = AttributeValue::S(nanoid!(10, &alphabet));
    let message_av = AttributeValue::S(text[5..].into());

    let request = dynamodb_client
      .put_item()
      .table_name(table_name)
      .item("id", id_av)
      .item("message", message_av)
      .send()
      .await;

    if let Err(error) = request {
      println!("Error trying put item to dynamodb");
      println!("{:?}", error);
    }

    let message_data = json!({
        "chat_id": message.chat.id,
        "text": "message added successfully",
    });
  
    let res = client.post(send_message_url)
        .header(CONTENT_TYPE, "application/json")
        .body(message_data.to_string())
        .send()
        .await;
    
    if let Err(error) = res {
        println!("Error trying to send message after add message to dynamodb");
        println!("{:?}", error);
    }

    return Ok(());
  }
  if is_message_from_admin && text == "/list" {

    let request = dynamodb_client
      .scan()
      .table_name(table_name)
      .send()
      .await;

    match request {
      Err(error) => {
        println!("Error trying list items from dynamodb");
        println!("{:?}", error);
        return Ok(());
      },
      Ok(response) => {
        match response.items {
          Some(items) => {
    
            println!("items: {:?}", items);

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

            println!("{:?}", message_text);

            let message_data = json!({
                "chat_id": message.chat.id,
                "text": message_text,
                "parse_mode": "MarkdownV2",
            });

            println!("{:?}", message_data);
      
            let request = client.post(send_message_url)
                .header(CONTENT_TYPE, "application/json")
                .body(message_data.to_string())
                .send()
                .await;

            println!("{:?}", request);

            match request {
              Ok(response) => {
                let body_result = response.text().await;

                match body_result {
                  Ok(body) => {
                    println!("{:?}", body);
                  },
                  Err(error) => {
                    println!("Error trying to get body from request");
                    println!("{:?}", error);
                  }
                }
              },
              Err(error) => {
                println!("Error trying to send message after list messages from dynamodb");
                println!("{:?}", error);
              }
            }
        
            return Ok(());
          },
          _ => {
            let message_data = json!({
                "chat_id": message.chat.id,
                "text": "There are no messages registered.",
            });
      
            let res = client.post(send_message_url)
                .header(CONTENT_TYPE, "application/json")
                .body(message_data.to_string())
                .send()
                .await;
            
            if let Err(error) = res {
                println!("Error trying to send message after list messages from dynamodb");
                println!("{:?}", error);
            }
        
            return Ok(());
          }
        }
      }
    }
  }
  if is_message_from_admin && text.starts_with("/rm") {
    println!("start_with match to /rm: |{}|", &text[3..]);

    let item_id = &text[3..];    

    let request = dynamodb_client
      .delete_item()
      .table_name(table_name)
      .key("id", AttributeValue::S(item_id.into()))
      .send()
      .await;

    if let Err(error) = request {
      println!("Error trying delete item to dynamodb");
      println!("{:?}", error);
    }
    let message_data = json!({
        "chat_id": message.chat.id,
        "text": "Item deleted successfully.",
    });

    let res = client.post(send_message_url)
        .header(CONTENT_TYPE, "application/json")
        .body(message_data.to_string())
        .send()
        .await;
    
    if let Err(error) = res {
        println!("Error trying to send message after delete message from dynamodb");
        println!("{:?}", error);
    }

    return Ok(());
  }
  
  let mut rng = rand::thread_rng();

  let should_not_send_message: bool = match message.chat.r#type {
    ChatType::Private => false,
    _ => rng.gen_bool(0.6),
  };

  if should_not_send_message {
      println!("Don't send message");
      return Ok(());
  }  

  let request = dynamodb_client
  .scan()
  .table_name(table_name)
  .send()
  .await;

  match request {
    Err(error) => {
      println!("Error trying list items from dynamodb");
      println!("{:?}", error);
    },
    Ok(response) => {
      match response.items {
        Some(items) => {

          if items.is_empty() {
            println!("There are no messages to send");
            return Ok(());
          }

          println!("items: {:?}", items);

          let message_to_send = items.choose(&mut rng).unwrap();

          let message_data = json!({
            "chat_id": message.chat.id,
            "text": message_to_send.get("message").unwrap().as_s().unwrap(),
          });
    
          let request = client.post(send_message_url)
              .header(CONTENT_TYPE, "application/json")
              .body(message_data.to_string())
              .send()
              .await;

          println!("{:?}", request);

          match request {
            Ok(response) => {
              let body_result = response.text().await;

              match body_result {
                Ok(body) => {
                  println!("{:?}", body);
                },
                Err(error) => {
                  println!("Error trying to get body from request");
                  println!("{:?}", error);
                }
              }
            },
            Err(error) => {
              println!("Error trying to send message after list messages from dynamodb");
              println!("{:?}", error);
            }
          }
        },
        _ => {
          let message_data = json!({
              "chat_id": message.chat.id,
              "text": "There are no messages registered.",
          });
    
          let res = client.post(send_message_url)
              .header(CONTENT_TYPE, "application/json")
              .body(message_data.to_string())
              .send()
              .await;
          
          if let Err(error) = res {
              println!("Error trying to send message after list messages from dynamodb");
              println!("{:?}", error);
          }
        }
      }
    }
  }

  Ok(())
}
