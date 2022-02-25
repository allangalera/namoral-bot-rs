terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 4.0.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.1.0"
    }
    archive = {
      source  = "hashicorp/archive"
      version = "~> 2.2.0"
    }
    local = {
      source  = "hashicorp/local"
      version = "~> 2.1.0"
    }
  }

  required_version = "~> 1.0"
}

provider "aws" {
  region = var.aws_region
}

resource "random_id" "random_path" {
  byte_length = 16
}

resource "random_id" "random_sufix" {
  byte_length = 8
  prefix      = "${var.project_name}-${var.stage}-"
}

resource "aws_s3_bucket" "lambda_bucket" {
  bucket = random_id.random_sufix.hex

  force_destroy = true
}

# archive and upload to s3
data "archive_file" "lambda_function" {
  type = "zip"

  source_file = "${path.module}/target/release/bootstrap"
  output_path = "${path.module}/${var.project_name}-${var.stage}.zip"
}

resource "aws_s3_object" "lambda_function" {
  bucket = aws_s3_bucket.lambda_bucket.id

  key    = "${var.project_name}-${var.stage}.zip"
  source = data.archive_file.lambda_function.output_path

  etag = filemd5(data.archive_file.lambda_function.output_path)
}

# Create dynamoDb Table

resource "aws_dynamodb_table" "table" {
  name           = random_id.random_sufix.hex
  hash_key       = "id"
  billing_mode   = "PROVISIONED"
  read_capacity  = 5
  write_capacity = 5
  attribute {
    name = "id"
    type = "S"
  }
}

# Create aws lambda

resource "aws_lambda_function" "lambda" {
  function_name = random_id.random_sufix.hex

  s3_bucket = aws_s3_bucket.lambda_bucket.id
  s3_key    = aws_s3_object.lambda_function.key

  runtime = "provided.al2"
  handler = "hello.handler"

  environment {
    variables = {
      domain          = aws_apigatewayv2_api.api.api_endpoint
      token_parameter = "${var.project_name}-${var.stage}-token"
      route_path      = random_id.random_path.hex
      table_name      = aws_dynamodb_table.table.name
      admin_id        = var.admin_user_id
    }
  }

  source_code_hash = data.archive_file.lambda_function.output_base64sha256

  role = aws_iam_role.lambda_exec.arn
}

data "aws_lambda_invocation" "set_webhook" {
  function_name = aws_lambda_function.lambda.function_name

  input = <<JSON
{
	"set_webhook": true
}
JSON
}

resource "aws_cloudwatch_log_group" "hello_world" {
  name              = "/aws/lambda/${aws_lambda_function.lambda.function_name}"
  retention_in_days = 7
}

data "aws_iam_policy_document" "lambda_exec_role_policy" {
  statement {
    actions = [
      "logs:CreateLogStream",
      "logs:PutLogEvents"
    ]
    resources = [
      "arn:aws:logs:*:*:*"
    ]
  }
  statement {
    actions = [
      "ssm:GetParameter",
    ]
    resources = [
      "arn:aws:ssm:us-east-1:553441724373:parameter/${var.project_name}-${var.stage}-token"
    ]
  }
  statement {
    actions = [
      "dynamodb:Scan",
      "dynamodb:PutItem",
      "dynamodb:DeleteItem",
    ]
    resources = [
      aws_dynamodb_table.table.arn
    ]
  }
}

resource "aws_iam_role_policy" "lambda_exec_role" {
  role   = aws_iam_role.lambda_exec.id
  policy = data.aws_iam_policy_document.lambda_exec_role_policy.json
}

resource "aws_iam_role" "lambda_exec" {
  name = random_id.random_sufix.hex

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Sid    = ""
      Principal = {
        Service = "lambda.amazonaws.com"
      }
    }]
  })
}

# Create API Gateway

resource "aws_apigatewayv2_api" "api" {
  name          = random_id.random_sufix.hex
  protocol_type = "HTTP"
}

resource "aws_apigatewayv2_integration" "api" {
  api_id = aws_apigatewayv2_api.api.id

  integration_uri        = aws_lambda_function.lambda.invoke_arn
  integration_type       = "AWS_PROXY"
  integration_method     = "POST"
  payload_format_version = "2.0"
}

resource "aws_apigatewayv2_route" "api" {
  api_id = aws_apigatewayv2_api.api.id

  route_key = "ANY /${random_id.random_path.hex}/{proxy+}"
  target    = "integrations/${aws_apigatewayv2_integration.api.id}"
}

resource "aws_apigatewayv2_stage" "api" {
  api_id      = aws_apigatewayv2_api.api.id
  name        = "$default"
  auto_deploy = true
}

resource "aws_cloudwatch_log_group" "api_gw" {
  name = "/aws/api_gw/${aws_apigatewayv2_api.api.name}"

  retention_in_days = 7
}

resource "aws_lambda_permission" "api_gw" {
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.lambda.arn
  principal     = "apigateway.amazonaws.com"

  source_arn = "${aws_apigatewayv2_api.api.execution_arn}/*/*"
}

