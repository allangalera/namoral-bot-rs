
variable "aws_region" {
  description = "AWS region for all resources."

  type     = string
  default  = "us-east-1"
  nullable = false
}

variable "project_name" {
  description = "This is going to be the prefix of all aws resources"

  type     = string
  default  = "namoral-bot"
  nullable = false
}

variable "stage" {
  description = "Stage to deploy. Ex.: dev, stage, prod. Default is 'dev'"

  type     = string
  default  = "prd"
  nullable = false
}

variable "admin_user_id" {
  description = "The user id that is going to be able to add, view and delete messages"

  type     = number
  nullable = false
}
