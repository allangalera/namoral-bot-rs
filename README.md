# namoral-bot rust

Implementation of my namoral-bot in rust as a way to learn rust

## What does it do?

The purpose of the bot is:
- when in a private chat: send a random message from a list of messages
- when in group chat: check a random percentage if it's going to send the message or not. If yes, send a random message from a list of messages

## How does it do?

In a private chat with the bot the user can use a few commands:
- `/add <phrase>`: add a phrase to the database
- `/rm<phrase_id>`: removes a phrase from the database
- `/list`: list the phrases from the database
  
## What does it uses?

All bot infra is built on AWS. To deploy I created a [terraform](https://www.terraform.io/) code to make it easy to deploy.

It uses x AWS resources:
- [Dynamodb](https://aws.amazon.com/dynamodb/)
- [Lambda](https://aws.amazon.com/lambda/)
- [API Gateway](https://aws.amazon.com/api-gateway/)
- [Cloudwatch](https://aws.amazon.com/cloudwatch/)
- [AWS Secrets Manager Parameter Store](https://docs.aws.amazon.com/systems-manager/latest/userguide/systems-manager-parameter-store.html)

## I want to deploy what do I do?

1. You have to go to [BotFather](https://telegram.me/BotFather) and create a bot. You can follow [this](https://core.telegram.org/bots#6-botfather) tutorial. This page also has other information about bots.
2. Go to AWS Secrets Manager Parameter Store and create a secret named `namoral-bot-token`. If you want to change the name of the token go to `main.tf` file and change the variable `token_parameter` that goes to the lambda function.
3. run `cargo build --release` to build the application
4. run `terraform plan` to validate your terraform code
5. run `terraform apply` to deploy it to aws
6. start using the bot
   
## I deployed how do I delete?

With terraform you can run `terraform destroy` and it will remove all infra it created. It's importante to keep the files that it was generated on the `terraform apply` so that it knows what resources it need to destroy.

## I want to change the name of the resources from aws

There is a resource on `main.tf` file that is called `random_sufix`. Change the sufix to whatever you want.

## You have all code in two files. It's a mess

Yeah. Calm down. I'm still learning rust so I still don't know how I should create things.

## Everyone can add/list/remove the messages on your bot?

Yeah. This is being adderessed on my next commit. I'll add another variable to lambda so that it can be configured the user with rights to edit.