resource "google_artifact_registry_repository" "images" {
  project       = var.project_id
  location      = var.region
  repository_id = var.name
  format        = "DOCKER"
  description   = "Docker images for ${var.name}"

  cleanup_policy_dry_run = false

  # A keep policy only protects images from delete policies; on its own it deletes nothing.
  cleanup_policies {
    id     = "delete-older-than-1-day"
    action = "DELETE"
    condition {
      tag_state  = "ANY"
      older_than = "1d"
    }
  }

  cleanup_policies {
    id     = "keep-3-most-recent"
    action = "KEEP"
    most_recent_versions {
      keep_count = 3
    }
  }
}

# The value is added by hand: a value managed by Terraform would be stored in its state.
resource "google_secret_manager_secret" "database_url" {
  project             = var.project_id
  secret_id           = "${var.name}-database-url"
  deletion_protection = true

  replication {
    user_managed {
      replicas {
        location = var.region
      }
    }
  }
}
