output "random_sufix" {
  description = "Random name for all resources"
  value       = random_id.random_sufix.hex
}

output "bucket" {
  description = "Name of the S3 bucket used to store function code."
  value       = aws_s3_bucket.lambda_bucket.bucket
}

output "lambda" {
  description = "Name of the lambda function."
  value       = aws_lambda_function.lambda.function_name
}

output "api_gateway_domain" {
  description = "Url of the resulting api url"
  value       = aws_apigatewayv2_api.api.api_endpoint
}

output "api_gateway_endpoint" {
  description = "Url of the resulting api route"
  value       = aws_apigatewayv2_route.api.route_key
}
