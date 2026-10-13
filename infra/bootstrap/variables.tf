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

variable "state_versions_kept" {
  description = "State versions to keep, including the current one."
  type        = number
  default     = 10
}
