mock_provider "google" {
  # Computed attributes are random strings by default; the IAM resources validate their format.
  mock_resource "google_service_account" {
    defaults = {
      name   = "projects/test-project/serviceAccounts/mock-account@test-project.iam.gserviceaccount.com"
      email  = "mock-account@test-project.iam.gserviceaccount.com"
      member = "serviceAccount:mock-account@test-project.iam.gserviceaccount.com"
    }
  }
}

variables {
  project_id           = "test-project"
  region               = "europe-west3"
  name                 = "smoke-tracker"
  github_repository    = "locadani/smoke-tracker"
  github_repository_id = "1416787003"
}

run "registry_deletes_old_images_but_keeps_three" {
  assert {
    condition     = google_artifact_registry_repository.images.format == "DOCKER" && google_artifact_registry_repository.images.location == "europe-west3"
    error_message = "The registry must be a Docker repository in the app's region."
  }
  assert {
    condition     = google_artifact_registry_repository.images.cleanup_policy_dry_run == false
    error_message = "Cleanup must actually delete (dry run off)."
  }
  assert {
    condition     = anytrue([for p in google_artifact_registry_repository.images.cleanup_policies : p.action == "DELETE" && try(p.condition[0].older_than, "") == "1d"])
    error_message = "A delete policy is required: a keep policy alone deletes nothing."
  }
  assert {
    condition     = anytrue([for p in google_artifact_registry_repository.images.cleanup_policies : p.action == "KEEP" && try(p.most_recent_versions[0].keep_count, 0) == 3])
    error_message = "The 3 most recent images must be kept for rollbacks."
  }
}

run "secret_is_regional_protected_and_has_no_value" {
  assert {
    condition     = google_secret_manager_secret.database_url.secret_id == "smoke-tracker-database-url"
    error_message = "The secret is named after the app."
  }
  assert {
    condition     = google_secret_manager_secret.database_url.replication[0].user_managed[0].replicas[0].location == "europe-west3"
    error_message = "The secret must be stored only in the app's region."
  }
  assert {
    condition     = google_secret_manager_secret.database_url.deletion_protection == true
    error_message = "The secret must be protected against deletion."
  }
}

run "name_must_fit_account_ids" {
  command = plan

  variables {
    name = "a-very-long-application-name"
  }

  expect_failures = [var.name]
}

run "runtime_account_can_only_read_its_secret" {
  assert {
    condition     = google_service_account.runtime.account_id == "smoke-tracker-run"
    error_message = "The runtime account is named <name>-run."
  }
  assert {
    condition     = google_secret_manager_secret_iam_member.runtime_reads_database_url.role == "roles/secretmanager.secretAccessor" && google_secret_manager_secret_iam_member.runtime_reads_database_url.member == google_service_account.runtime.member
    error_message = "The runtime account must be able to read the database secret."
  }
  assert {
    condition     = google_secret_manager_secret_iam_member.runtime_reads_database_url.secret_id == google_secret_manager_secret.database_url.secret_id
    error_message = "The grant must be on the one secret, not the project."
  }
}

run "deploy_account_can_push_images_and_act_as_runtime" {
  assert {
    condition     = google_service_account.deploy.account_id == "smoke-tracker-deploy"
    error_message = "The deploy account is named <name>-deploy."
  }
  assert {
    condition     = google_artifact_registry_repository_iam_member.deploy_pushes_images.role == "roles/artifactregistry.writer" && google_artifact_registry_repository_iam_member.deploy_pushes_images.member == google_service_account.deploy.member
    error_message = "The deploy account must be able to push to the app's registry only."
  }
  assert {
    condition     = google_service_account_iam_member.deploy_acts_as_runtime.role == "roles/iam.serviceAccountUser" && google_service_account_iam_member.deploy_acts_as_runtime.service_account_id == google_service_account.runtime.name
    error_message = "The deploy account must be able to run revisions as the runtime account."
  }
}
