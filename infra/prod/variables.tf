variable "project_id" {
  description = "Google Cloud project ID (kept out of the repository; see terraform.tfvars.example)."
  type        = string
}

variable "state_bucket" {
  description = "Bucket that holds Terraform state, created by hand before the first run."
  type        = string
}

variable "region" {
  description = "Region for every regional resource."
  type        = string
  default     = "europe-west3"
}
