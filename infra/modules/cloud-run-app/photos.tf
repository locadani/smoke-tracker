resource "google_storage_bucket" "photos" {
  project                     = var.project_id
  name                        = "${var.name}-photos-${var.project_id}"
  location                    = upper(var.region)
  force_destroy               = false
  uniform_bucket_level_access = true
  public_access_prevention    = "enforced"

  # Only uploads that never got attached to a reading are removed; stored photos are evidence and stay.
  lifecycle_rule {
    action {
      type = "Delete"
    }
    condition {
      age            = 1
      matches_prefix = ["pending/"]
    }
  }
}

# Create and read, never delete or overwrite: a stored photo cannot be altered by the app.
resource "google_storage_bucket_iam_member" "runtime_creates_photos" {
  bucket = google_storage_bucket.photos.name
  role   = "roles/storage.objectCreator"
  member = google_service_account.runtime.member
}

resource "google_storage_bucket_iam_member" "runtime_reads_photos" {
  bucket = google_storage_bucket.photos.name
  role   = "roles/storage.objectViewer"
  member = google_service_account.runtime.member
}
