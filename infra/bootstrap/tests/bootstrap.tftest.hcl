mock_provider "google" {}

variables {
  project_id  = "test-project"
  bucket_name = "test-project-tfstate"
}

run "state_bucket_is_private_versioned_and_regional" {
  command = plan

  assert {
    condition     = google_storage_bucket.state.location == "EUROPE-WEST3"
    error_message = "The state bucket lives in the app's region."
  }
  assert {
    condition     = google_storage_bucket.state.public_access_prevention == "enforced" && google_storage_bucket.state.uniform_bucket_level_access == true
    error_message = "The state bucket must never be public."
  }
  assert {
    condition     = google_storage_bucket.state.versioning[0].enabled == true
    error_message = "Versioning keeps older state files for recovery."
  }
  assert {
    condition     = google_storage_bucket.state.force_destroy == false
    error_message = "Terraform must not empty the bucket to delete it."
  }
}

run "old_state_versions_are_pruned" {
  command = plan

  assert {
    condition     = anytrue([for r in google_storage_bucket.state.lifecycle_rule : tolist(r.action)[0].type == "Delete" && tolist(r.condition)[0].num_newer_versions == 10 && tolist(r.condition)[0].with_state == "ARCHIVED"])
    error_message = "Only older copies beyond the newest 10 are deleted."
  }
}
