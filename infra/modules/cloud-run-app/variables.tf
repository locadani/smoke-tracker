variable "project_id" {
  description = "Google Cloud project that hosts the app."
  type        = string
}

variable "region" {
  description = "Region for the service, image registry and secret."
  type        = string
}

variable "name" {
  description = "App name; prefixes every resource name."
  type        = string

  validation {
    condition     = can(regex("^[a-z][a-z0-9-]{3,21}[a-z0-9]$", var.name))
    error_message = "name must be 5-23 characters of a-z, 0-9 and '-', starting with a letter and ending with a letter or digit, so that '<name>-deploy' fits a 30-character service account ID."
  }
}

variable "github_repository" {
  description = "GitHub repository as owner/name, used to match the deploy workflow."
  type        = string
}

variable "github_repository_id" {
  description = "Numeric GitHub repository ID; only this repository may log in."
  type        = string
}

variable "deploy_workflow" {
  description = "Workflow file that may deploy, from the main branch only."
  type        = string
  default     = "deploy.yml"
}

variable "attach_database_url" {
  description = "Give the service DATABASE_URL from the secret. Enable only after the secret has a value: Cloud Run won't start an instance whose secret can't be read."
  type        = bool
  default     = false
}

variable "initial_image" {
  description = "Image used when the service is created; later images come from the deploy workflow."
  type        = string
  default     = "us-docker.pkg.dev/cloudrun/container/hello"
}

variable "memory" {
  description = "Memory per instance. Below 512Mi Cloud Run requires the first-generation runtime."
  type        = string
  default     = "512Mi"
}

variable "max_instances" {
  description = "Upper bound on running instances, which caps compute cost."
  type        = number
  default     = 1
}
