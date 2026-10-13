variable "project_id" {
  description = "Google Cloud project ID (kept out of the repository; see terraform.tfvars.example)."
  type        = string
}

variable "bucket_name" {
  description = "Globally unique name for the Terraform state bucket, e.g. <project-id>-tfstate."
  type        = string
}

variable "region" {
  description = "Region of the state bucket."
  type        = string
  default     = "europe-west3"
}

variable "noncurrent_versions_kept" {
  description = "Older copies of each state file to keep, for recovery."
  type        = number
  default     = 10
}
