# Hosting Infrastructure (Terraform) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Describe Smoke Tracker's Google Cloud hosting (Cloud Run, Artifact Registry, Secret Manager, service accounts, GitHub keyless login) as Terraform in `infra/`, checked and planned on every PR.

**Architecture:** A reusable module `infra/modules/cloud-run-app` describes one app; the root `infra/prod` enables project APIs, calls the module for Smoke Tracker, adopts the hand-made Artifact Registry repository and creates the read-only `terraform-plan` account. A GitHub workflow `infra.yml` formats, validates, tests (mocked provider), scans and plans; the user applies by hand in Cloud Shell.

**Tech Stack:** Terraform 1.16.5, `hashicorp/google` provider 8.6, `terraform test` with `mock_provider`, GitHub Actions (`hashicorp/setup-terraform@v4`, `google-github-actions/auth@v3`, `actions/github-script@v9`, `aquasecurity/trivy-action` v0.36.0 pinned by SHA), Trivy config scanning.

**Spec:** `docs/specs/2026-10-13-hosting-infra-design.md`

## Global Constraints

- Terraform `1.16.5` (root `required_version = "~> 1.16.0"`); provider `hashicorp/google` `~> 8.6` in the root, `>= 8.0` in the module.
- Region `europe-west3` everywhere. App name `smoke-tracker`. GitHub repository `locadani/smoke-tracker`, repository ID `1416787003`.
- No secret value anywhere in Terraform, the repository, CI logs or PR comments. The database connection string is never written by Terraform.
- The project ID and state bucket name are never committed: `infra/prod/terraform.tfvars` and `infra/prod/backend.hcl` are gitignored; CI reads them from GitHub **secrets**.
- Nothing is installed on the development machine. Run Terraform through Docker: `docker run --rm -v "$PWD:/repo" -w /repo/<dir> hashicorp/terraform:1.16.5 <args>` from the worktree root. Never `terraform apply` or `plan` against Google from this machine (no credentials here); only `init -backend=false`, `fmt`, `validate`, `test`, `providers lock`.
- Commits: as `locadani` (repo-local config already set), concept-level messages, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Never push; the controller pushes.
- No tutorial comments in code; teaching material goes in `docs/learning/`. Short comments only where a reader would ask "why?".
- Never modify `claude.md`, `description.md`, or the original design spec `docs/specs/2026-10-13-smoke-tracker-design.md`.

## Review Focus

1. **First `apply` with an empty secret** — the service must be created and start (sample image, no `DATABASE_URL`). Pinned by the Task 3 test `database_url_not_attached_by_default`.
2. **Deploy changes the image** — the next `plan` must not roll it back. Pinned by Task 3's `lifecycle.ignore_changes` (reviewed; verified for real after the first deploy, spec §9).
3. **Plan job on Dependabot, fork, or pre-setup PRs** — must be skipped, not failed. Pinned by Task 6's `gate` job.
4. **IDs leaking into the public PR comment** — the posted text must have the project ID and number replaced. Pinned by Task 6's `redact-plan.test.sh`.
5. **App name too long or invalid for account IDs** (`<name>-deploy` must be ≤ 30 characters) — must fail at plan time with a clear message. Pinned by Task 1's `name_must_fit_account_ids` test.

---

## File map

| File | Responsibility |
|---|---|
| `.gitignore` | Terraform local files, state, and the two private config files |
| `infra/modules/cloud-run-app/versions.tf` | provider requirement |
| `infra/modules/cloud-run-app/variables.tf` | module inputs and their validation |
| `infra/modules/cloud-run-app/main.tf` | Artifact Registry, secret, service accounts, their grants |
| `infra/modules/cloud-run-app/service.tf` | Cloud Run service and the deploy account's grant on it |
| `infra/modules/cloud-run-app/github-login.tf` | Workload Identity pool, provider, deploy binding |
| `infra/modules/cloud-run-app/outputs.tf` | values for `prod/` and for GitHub settings |
| `infra/modules/cloud-run-app/tests/cloud_run_app.tftest.hcl` | module tests (mocked provider) |
| `infra/prod/versions.tf`, `backend.tf`, `providers.tf`, `variables.tf` | root setup |
| `infra/prod/project.tf` | project APIs, project data |
| `infra/prod/main.tf` | the module call with Smoke Tracker's values |
| `infra/prod/imports.tf` | adopt the hand-made Artifact Registry repository |
| `infra/prod/ci.tf` | read-only `terraform-plan` account |
| `infra/prod/outputs.tf` | values to copy into GitHub |
| `infra/prod/terraform.tfvars.example`, `backend.hcl.example` | templates for the private files |
| `infra/prod/.terraform.lock.hcl` | provider checksums (generated) |
| `infra/scripts/redact-plan.sh`, `redact-plan.test.sh` | replace IDs in plan text; its test |
| `.github/workflows/infra.yml` | check / gate / plan jobs |
| `.github/dependabot.yml` | add the `terraform` ecosystem |
| `infra/README.md` | manual vs Terraform, first-time setup, routine |
| `README.md`, `docs/learning/hosting.md` | updated hosting docs |
| `docs/DECISIONS.md` | new decision entries (user approval required) |

---

### Task 1: Module scaffold, Artifact Registry and secret

**Files:**
- Modify: `.gitignore`
- Create: `infra/modules/cloud-run-app/versions.tf`, `variables.tf`, `main.tf`, `tests/cloud_run_app.tftest.hcl`

**Interfaces:**
- Produces: module variables `project_id`, `region`, `name`, `github_repository`, `github_repository_id`, `deploy_workflow`, `attach_database_url`, `initial_image`, `memory`, `max_instances`; resources `google_artifact_registry_repository.images`, `google_secret_manager_secret.database_url`.

- [ ] **Step 1: Ignore Terraform local files**

Append to `.gitignore`:

```gitignore

# Terraform
.terraform/
*.tfstate
*.tfstate.*
*.tfplan
crash.log
infra/prod/terraform.tfvars
infra/prod/backend.hcl
infra/prod/plan*.txt
infra/modules/**/.terraform.lock.hcl
```

- [ ] **Step 2: Write the failing tests**

Create `infra/modules/cloud-run-app/tests/cloud_run_app.tftest.hcl`:

```hcl
mock_provider "google" {}

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
```

- [ ] **Step 3: Run the tests to verify they fail**

Run (from the worktree root):
```bash
docker run --rm -v "$PWD:/repo" -w /repo/infra/modules/cloud-run-app hashicorp/terraform:1.16.5 init -backend=false -input=false
docker run --rm -v "$PWD:/repo" -w /repo/infra/modules/cloud-run-app hashicorp/terraform:1.16.5 test
```
Expected: `init` fails or `test` fails with errors such as `Reference to undeclared resource` / `undeclared input variable` (no module code yet).

- [ ] **Step 4: Write the module scaffold and resources**

Create `infra/modules/cloud-run-app/versions.tf`:

```hcl
terraform {
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = ">= 8.0"
    }
  }
}
```

Create `infra/modules/cloud-run-app/variables.tf`:

```hcl
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
    condition     = can(regex("^[a-z][a-z0-9-]{4,22}$", var.name))
    error_message = "name must be 5-23 characters of a-z, 0-9 and '-', starting with a letter, so that '<name>-deploy' fits a 30-character service account ID."
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
```

Create `infra/modules/cloud-run-app/main.tf`:

```hcl
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
```

- [ ] **Step 5: Run the tests to verify they pass**

Run:
```bash
docker run --rm -v "$PWD:/repo" -w /repo/infra/modules/cloud-run-app hashicorp/terraform:1.16.5 init -backend=false -input=false
docker run --rm -v "$PWD:/repo" -w /repo/infra/modules/cloud-run-app hashicorp/terraform:1.16.5 test
docker run --rm -v "$PWD:/repo" -w /repo hashicorp/terraform:1.16.5 fmt -check -recursive infra
```
Expected: `Success! 3 passed, 0 failed.` and `fmt` prints nothing.

- [ ] **Step 6: Commit**

```bash
git add .gitignore infra/modules/cloud-run-app
git commit -m "infra: cloud-run-app module with image registry and database secret

The registry deletes images older than a day but keeps the 3 most recent;
the secret is regional, deletion-protected and has no value in Terraform.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Service accounts and their grants

**Files:**
- Modify: `infra/modules/cloud-run-app/main.tf`
- Modify: `infra/modules/cloud-run-app/tests/cloud_run_app.tftest.hcl`

**Interfaces:**
- Consumes: `google_artifact_registry_repository.images`, `google_secret_manager_secret.database_url` (Task 1).
- Produces: `google_service_account.runtime` (`<name>-run`), `google_service_account.deploy` (`<name>-deploy`), `google_secret_manager_secret_iam_member.runtime_reads_database_url`, `google_artifact_registry_repository_iam_member.deploy_pushes_images`, `google_service_account_iam_member.deploy_acts_as_runtime`.

- [ ] **Step 1: Write the failing tests**

Append to `tests/cloud_run_app.tftest.hcl`:

```hcl
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `docker run --rm -v "$PWD:/repo" -w /repo/infra/modules/cloud-run-app hashicorp/terraform:1.16.5 test`
Expected: FAIL with `Reference to undeclared resource` for `google_service_account.runtime`.

- [ ] **Step 3: Write the accounts and grants**

Append to `infra/modules/cloud-run-app/main.tf`:

```hcl
resource "google_service_account" "runtime" {
  project      = var.project_id
  account_id   = "${var.name}-run"
  display_name = "${var.name} on Cloud Run"
}

resource "google_service_account" "deploy" {
  project      = var.project_id
  account_id   = "${var.name}-deploy"
  display_name = "${var.name} deploys from GitHub Actions"
}

resource "google_secret_manager_secret_iam_member" "runtime_reads_database_url" {
  project   = var.project_id
  secret_id = google_secret_manager_secret.database_url.secret_id
  role      = "roles/secretmanager.secretAccessor"
  member    = google_service_account.runtime.member
}

resource "google_artifact_registry_repository_iam_member" "deploy_pushes_images" {
  project    = var.project_id
  location   = var.region
  repository = google_artifact_registry_repository.images.name
  role       = "roles/artifactregistry.writer"
  member     = google_service_account.deploy.member
}

resource "google_service_account_iam_member" "deploy_acts_as_runtime" {
  service_account_id = google_service_account.runtime.name
  role               = "roles/iam.serviceAccountUser"
  member             = google_service_account.deploy.member
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `docker run --rm -v "$PWD:/repo" -w /repo/infra/modules/cloud-run-app hashicorp/terraform:1.16.5 test` and `docker run --rm -v "$PWD:/repo" -w /repo hashicorp/terraform:1.16.5 fmt -check -recursive infra`
Expected: `Success! 5 passed, 0 failed.`; `fmt` prints nothing.

- [ ] **Step 5: Commit**

```bash
git add infra/modules/cloud-run-app
git commit -m "infra: runtime and deploy service accounts with least-privilege grants

Each grant is on a single resource: the runtime account reads one secret;
the deploy account pushes to one registry and acts as the runtime account.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Cloud Run service

**Files:**
- Create: `infra/modules/cloud-run-app/service.tf`
- Modify: `infra/modules/cloud-run-app/tests/cloud_run_app.tftest.hcl`

**Interfaces:**
- Consumes: `google_service_account.runtime`, `google_service_account.deploy`, `google_secret_manager_secret.database_url`, `google_secret_manager_secret_iam_member.runtime_reads_database_url` (Tasks 1–2); variables `attach_database_url`, `initial_image`, `memory`, `max_instances`.
- Produces: `google_cloud_run_v2_service.app`, `google_cloud_run_v2_service_iam_member.deploy_updates_service`.

- [ ] **Step 1: Write the failing tests**

Append to `tests/cloud_run_app.tftest.hcl`:

```hcl
run "service_is_public_capped_and_protected" {
  assert {
    condition     = google_cloud_run_v2_service.app.invoker_iam_disabled == true
    error_message = "The service is public through invoker_iam_disabled, not an allUsers grant."
  }
  assert {
    condition     = google_cloud_run_v2_service.app.template[0].scaling[0].min_instance_count == 0 && google_cloud_run_v2_service.app.template[0].scaling[0].max_instance_count == 1
    error_message = "The service scales to zero and to at most one instance."
  }
  assert {
    condition     = google_cloud_run_v2_service.app.template[0].containers[0].resources[0].cpu_idle == true
    error_message = "CPU must be allocated only during requests (free tier)."
  }
  assert {
    condition     = google_cloud_run_v2_service.app.template[0].containers[0].resources[0].limits["memory"] == "512Mi"
    error_message = "Memory defaults to 512Mi."
  }
  assert {
    condition     = google_cloud_run_v2_service.app.template[0].service_account == google_service_account.runtime.email
    error_message = "The service runs as the runtime account."
  }
  assert {
    condition     = google_cloud_run_v2_service.app.deletion_protection == true
    error_message = "The service must be protected against deletion."
  }
  assert {
    condition     = google_cloud_run_v2_service_iam_member.deploy_updates_service.role == "roles/run.developer" && google_cloud_run_v2_service_iam_member.deploy_updates_service.member == google_service_account.deploy.member
    error_message = "The deploy account may deploy this one service, and nothing more."
  }
}

run "database_url_not_attached_by_default" {
  assert {
    condition     = length(google_cloud_run_v2_service.app.template[0].containers[0].env) == 0
    error_message = "Before the secret has a value, the service must not reference it, or instances won't start."
  }
}

run "database_url_attached_from_secret_when_enabled" {
  variables {
    attach_database_url = true
  }

  assert {
    condition     = google_cloud_run_v2_service.app.template[0].containers[0].env[0].name == "DATABASE_URL"
    error_message = "The app reads its connection string as DATABASE_URL."
  }
  assert {
    condition     = google_cloud_run_v2_service.app.template[0].containers[0].env[0].value_source[0].secret_key_ref[0].secret == google_secret_manager_secret.database_url.secret_id && google_cloud_run_v2_service.app.template[0].containers[0].env[0].value_source[0].secret_key_ref[0].version == "latest"
    error_message = "DATABASE_URL comes from the latest version of the app's secret."
  }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `docker run --rm -v "$PWD:/repo" -w /repo/infra/modules/cloud-run-app hashicorp/terraform:1.16.5 test`
Expected: FAIL with `Reference to undeclared resource` for `google_cloud_run_v2_service.app`.

- [ ] **Step 3: Write the service**

Create `infra/modules/cloud-run-app/service.tf`:

```hcl
resource "google_cloud_run_v2_service" "app" {
  project              = var.project_id
  name                 = var.name
  location             = var.region
  ingress              = "INGRESS_TRAFFIC_ALL"
  invoker_iam_disabled = true
  deletion_protection  = true

  template {
    service_account = google_service_account.runtime.email

    scaling {
      min_instance_count = 0
      max_instance_count = var.max_instances
    }

    containers {
      image = var.initial_image

      resources {
        limits = {
          cpu    = "1"
          memory = var.memory
        }
        cpu_idle = true
      }

      dynamic "env" {
        for_each = var.attach_database_url ? [google_secret_manager_secret.database_url.secret_id] : []
        content {
          name = "DATABASE_URL"
          value_source {
            secret_key_ref {
              secret  = env.value
              version = "latest"
            }
          }
        }
      }
    }
  }

  lifecycle {
    # The deploy workflow owns the running image; without this, every apply would roll it back.
    ignore_changes = [
      template[0].containers[0].image,
      client,
      client_version,
    ]
  }

  depends_on = [google_secret_manager_secret_iam_member.runtime_reads_database_url]
}

resource "google_cloud_run_v2_service_iam_member" "deploy_updates_service" {
  project  = var.project_id
  location = var.region
  name     = google_cloud_run_v2_service.app.name
  role     = "roles/run.developer"
  member   = google_service_account.deploy.member
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `docker run --rm -v "$PWD:/repo" -w /repo/infra/modules/cloud-run-app hashicorp/terraform:1.16.5 test` and `docker run --rm -v "$PWD:/repo" -w /repo hashicorp/terraform:1.16.5 fmt -check -recursive infra`
Expected: `Success! 8 passed, 0 failed.`; `fmt` prints nothing.

- [ ] **Step 5: Commit**

```bash
git add infra/modules/cloud-run-app
git commit -m "infra: Cloud Run service, public, scale-to-zero, at most one instance

DATABASE_URL is attached from Secret Manager only once the secret has a
value; the deploy workflow owns the image, so Terraform ignores it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: GitHub keyless login and module outputs

**Files:**
- Create: `infra/modules/cloud-run-app/github-login.tf`, `infra/modules/cloud-run-app/outputs.tf`
- Modify: `infra/modules/cloud-run-app/tests/cloud_run_app.tftest.hcl`

**Interfaces:**
- Consumes: `google_service_account.deploy`, `google_service_account.runtime`, `google_cloud_run_v2_service.app`, `google_artifact_registry_repository.images`, `google_secret_manager_secret.database_url`.
- Produces: `google_iam_workload_identity_pool.github`, `google_iam_workload_identity_pool_provider.github`, `google_service_account_iam_member.github_deploys`; outputs `service_name`, `service_url`, `runtime_sa_email`, `deploy_sa_email`, `repository_url`, `database_url_secret_id`, `workload_identity_pool_name`, `workload_identity_provider_name`.

- [ ] **Step 1: Write the failing tests**

Append to `tests/cloud_run_app.tftest.hcl`:

```hcl
run "only_this_repository_can_log_in" {
  assert {
    condition     = google_iam_workload_identity_pool_provider.github.attribute_condition == "assertion.repository_id == '1416787003'"
    error_message = "Only tokens from this repository (by ID, not name) may be exchanged."
  }
  assert {
    condition     = google_iam_workload_identity_pool_provider.github.oidc[0].issuer_uri == "https://token.actions.githubusercontent.com"
    error_message = "The provider trusts GitHub's OIDC issuer."
  }
  assert {
    condition     = google_iam_workload_identity_pool_provider.github.attribute_mapping["attribute.repository_id"] == "assertion.repository_id" && google_iam_workload_identity_pool_provider.github.attribute_mapping["attribute.job_workflow_ref"] == "assertion.job_workflow_ref"
    error_message = "repository_id and job_workflow_ref must be mapped so bindings can use them."
  }
}

run "only_the_deploy_workflow_on_main_can_deploy" {
  assert {
    condition     = google_service_account_iam_member.github_deploys.role == "roles/iam.workloadIdentityUser" && google_service_account_iam_member.github_deploys.service_account_id == google_service_account.deploy.name
    error_message = "GitHub may impersonate the deploy account only."
  }
  assert {
    condition     = endswith(google_service_account_iam_member.github_deploys.member, "/attribute.job_workflow_ref/locadani/smoke-tracker/.github/workflows/deploy.yml@refs/heads/main")
    error_message = "Only deploy.yml running on main may impersonate the deploy account."
  }
  assert {
    condition     = startswith(google_service_account_iam_member.github_deploys.member, "principalSet://iam.googleapis.com/")
    error_message = "The binding is a principalSet on the pool."
  }
}

run "outputs_for_github_settings" {
  assert {
    condition     = output.repository_url == "europe-west3-docker.pkg.dev/test-project/smoke-tracker"
    error_message = "The image repository URL is <region>-docker.pkg.dev/<project>/<name>."
  }
  assert {
    condition     = output.database_url_secret_id == "smoke-tracker-database-url"
    error_message = "The secret ID is exposed for the setup instructions."
  }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `docker run --rm -v "$PWD:/repo" -w /repo/infra/modules/cloud-run-app hashicorp/terraform:1.16.5 test`
Expected: FAIL with `Reference to undeclared resource` for `google_iam_workload_identity_pool_provider.github`.

- [ ] **Step 3: Write the login and outputs**

Create `infra/modules/cloud-run-app/github-login.tf`:

```hcl
resource "google_iam_workload_identity_pool" "github" {
  project                   = var.project_id
  workload_identity_pool_id = "github"
  display_name              = "GitHub Actions"
}

resource "google_iam_workload_identity_pool_provider" "github" {
  project                            = var.project_id
  workload_identity_pool_id          = google_iam_workload_identity_pool.github.workload_identity_pool_id
  workload_identity_pool_provider_id = "github-actions"
  display_name                       = "GitHub Actions OIDC"

  # The ID, not the name: a repository name can be re-registered by someone else.
  attribute_condition = "assertion.repository_id == '${var.github_repository_id}'"

  attribute_mapping = {
    "google.subject"             = "assertion.sub"
    "attribute.repository_id"    = "assertion.repository_id"
    "attribute.job_workflow_ref" = "assertion.job_workflow_ref"
  }

  oidc {
    issuer_uri = "https://token.actions.githubusercontent.com"
  }
}

resource "google_service_account_iam_member" "github_deploys" {
  service_account_id = google_service_account.deploy.name
  role               = "roles/iam.workloadIdentityUser"
  member             = "principalSet://iam.googleapis.com/${google_iam_workload_identity_pool.github.name}/attribute.job_workflow_ref/${var.github_repository}/.github/workflows/${var.deploy_workflow}@refs/heads/main"
}
```

Create `infra/modules/cloud-run-app/outputs.tf`:

```hcl
output "service_name" {
  description = "Cloud Run service name."
  value       = google_cloud_run_v2_service.app.name
}

output "service_url" {
  description = "Default HTTPS address of the service."
  value       = google_cloud_run_v2_service.app.uri
}

output "runtime_sa_email" {
  description = "Account the app runs as."
  value       = google_service_account.runtime.email
}

output "deploy_sa_email" {
  description = "Account the deploy workflow impersonates."
  value       = google_service_account.deploy.email
}

output "repository_url" {
  description = "Where the deploy workflow pushes images."
  value       = "${var.region}-docker.pkg.dev/${var.project_id}/${google_artifact_registry_repository.images.repository_id}"
}

output "database_url_secret_id" {
  description = "Secret that holds the database connection string (value added by hand)."
  value       = google_secret_manager_secret.database_url.secret_id
}

output "workload_identity_pool_name" {
  description = "Full name of the GitHub pool, for further principalSet bindings."
  value       = google_iam_workload_identity_pool.github.name
}

output "workload_identity_provider_name" {
  description = "Full provider name for google-github-actions/auth."
  value       = google_iam_workload_identity_pool_provider.github.name
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `docker run --rm -v "$PWD:/repo" -w /repo/infra/modules/cloud-run-app hashicorp/terraform:1.16.5 test`, then `... validate` in the same directory, then `fmt -check -recursive infra` from the root.
Expected: `Success! 11 passed, 0 failed.`; `Success! The configuration is valid.`; `fmt` prints nothing.

- [ ] **Step 5: Commit**

```bash
git add infra/modules/cloud-run-app
git commit -m "infra: keyless GitHub login restricted to this repository and deploy.yml on main

Workload Identity Federation exchanges GitHub's OIDC token for a short-lived
Google token; no service account key exists.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Production root

**Files:**
- Create: `infra/prod/versions.tf`, `backend.tf`, `providers.tf`, `variables.tf`, `project.tf`, `main.tf`, `imports.tf`, `ci.tf`, `outputs.tf`, `terraform.tfvars.example`, `backend.hcl.example`, `.terraform.lock.hcl` (generated)

**Interfaces:**
- Consumes: the module and its outputs (Tasks 1–4).
- Produces: root variables `project_id`, `state_bucket`, `region`; outputs `service_url`, `project_number`, `workload_identity_provider`, `deploy_service_account`, `plan_service_account`, `image_repository`, `database_url_secret`.

- [ ] **Step 1: Write the root files**

`infra/prod/versions.tf`:

```hcl
terraform {
  required_version = "~> 1.16.0"

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 8.6"
    }
  }
}
```

`infra/prod/backend.tf`:

```hcl
# The bucket name is not committed: pass it with -backend-config=backend.hcl (see backend.hcl.example).
terraform {
  backend "gcs" {
    prefix = "smoke-tracker/prod"
  }
}
```

`infra/prod/providers.tf`:

```hcl
provider "google" {
  project = var.project_id
  region  = var.region
}
```

`infra/prod/variables.tf`:

```hcl
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
```

`infra/prod/project.tf`:

```hcl
locals {
  apis = toset([
    "artifactregistry.googleapis.com",
    "cloudresourcemanager.googleapis.com",
    "iam.googleapis.com",
    "iamcredentials.googleapis.com",
    "run.googleapis.com",
    "secretmanager.googleapis.com",
    "serviceusage.googleapis.com",
    "storage.googleapis.com",
    "sts.googleapis.com",
  ])
}

resource "google_project_service" "apis" {
  for_each = local.apis

  service            = each.value
  disable_on_destroy = false
}

data "google_project" "this" {}
```

`infra/prod/main.tf`:

```hcl
module "app" {
  source = "../modules/cloud-run-app"

  project_id           = var.project_id
  region               = var.region
  name                 = "smoke-tracker"
  github_repository    = "locadani/smoke-tracker"
  github_repository_id = "1416787003"

  # Set to true in a PR once the secret has its first version (infra/README.md, first-time setup).
  attach_database_url = false

  depends_on = [google_project_service.apis]
}
```

`infra/prod/imports.tf`:

```hcl
# Created by hand in the console before Terraform; adopted instead of recreated.
# Once imported, this block has no further effect.
import {
  to = module.app.google_artifact_registry_repository.images
  id = "projects/${var.project_id}/locations/${var.region}/repositories/smoke-tracker"
}
```

`infra/prod/ci.tf`:

```hcl
# Read-only account for `terraform plan` in pull requests. Applying is done by hand.
resource "google_service_account" "terraform_plan" {
  account_id   = "terraform-plan"
  display_name = "Terraform plan (read-only, GitHub Actions)"

  depends_on = [google_project_service.apis]
}

resource "google_project_iam_member" "terraform_plan" {
  for_each = toset(["roles/viewer", "roles/iam.securityReviewer"])

  project = var.project_id
  role    = each.value
  member  = google_service_account.terraform_plan.member
}

resource "google_storage_bucket_iam_member" "terraform_plan_reads_state" {
  bucket = var.state_bucket
  role   = "roles/storage.objectViewer"
  member = google_service_account.terraform_plan.member
}

# Any workflow of this repository: a binding matches one exact value, and this account is read-only.
resource "google_service_account_iam_member" "terraform_plan_github" {
  service_account_id = google_service_account.terraform_plan.name
  role               = "roles/iam.workloadIdentityUser"
  member             = "principalSet://iam.googleapis.com/${module.app.workload_identity_pool_name}/attribute.repository_id/1416787003"
}
```

`infra/prod/outputs.tf`:

```hcl
output "service_url" {
  description = "Open this to check the service."
  value       = module.app.service_url
}

output "project_number" {
  description = "GitHub secret GCP_PROJECT_NUMBER."
  value       = data.google_project.this.number
}

output "workload_identity_provider" {
  description = "GitHub secret GCP_WIF_PROVIDER."
  value       = module.app.workload_identity_provider_name
}

output "deploy_service_account" {
  description = "GitHub secret GCP_DEPLOY_SA."
  value       = module.app.deploy_sa_email
}

output "plan_service_account" {
  description = "GitHub secret GCP_PLAN_SA."
  value       = google_service_account.terraform_plan.email
}

output "image_repository" {
  description = "Where deploy.yml pushes images."
  value       = module.app.repository_url
}

output "database_url_secret" {
  description = "Add the Neon connection string as this secret's first version."
  value       = module.app.database_url_secret_id
}
```

`infra/prod/terraform.tfvars.example`:

```hcl
# Copy to terraform.tfvars (gitignored) and fill in.
project_id   = "your-project-id"
state_bucket = "your-state-bucket"
```

`infra/prod/backend.hcl.example`:

```hcl
# Copy to backend.hcl (gitignored), then: terraform init -backend-config=backend.hcl
bucket = "your-state-bucket"
```

- [ ] **Step 2: Validate and lock providers for every platform that runs Terraform**

Run:
```bash
docker run --rm -v "$PWD:/repo" -w /repo/infra/prod hashicorp/terraform:1.16.5 init -backend=false -input=false
docker run --rm -v "$PWD:/repo" -w /repo/infra/prod hashicorp/terraform:1.16.5 validate
docker run --rm -v "$PWD:/repo" -w /repo/infra/prod hashicorp/terraform:1.16.5 providers lock -platform=linux_amd64 -platform=linux_arm64 -platform=darwin_arm64
docker run --rm -v "$PWD:/repo" -w /repo hashicorp/terraform:1.16.5 fmt -check -recursive infra
```
Expected: `Success! The configuration is valid.`; `.terraform.lock.hcl` lists `hashicorp/google` `8.6.x` with hashes for three platforms (Cloud Shell and CI are `linux_amd64`); `fmt` prints nothing.

- [ ] **Step 3: Confirm nothing private is staged**

Run: `git status --short infra/prod`
Expected: the `.tf` files, the two `.example` files and `.terraform.lock.hcl`; no `.terraform/`, no `terraform.tfvars`, no `backend.hcl`.

- [ ] **Step 4: Commit**

```bash
git add infra/prod
git commit -m "infra: production root — project APIs, the app module, read-only plan account

The hand-made image registry is adopted with an import block; the project
ID and state bucket stay out of the repository.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: CI — redaction script, `infra.yml`, Dependabot

**Files:**
- Create: `infra/scripts/redact-plan.sh`, `infra/scripts/redact-plan.test.sh`, `.github/workflows/infra.yml`
- Modify: `.github/dependabot.yml`

**Interfaces:**
- Consumes: `infra/prod` (Task 5), the module tests (Tasks 1–4).
- Produces: GitHub secrets read by CI: `GCP_PROJECT_ID`, `GCP_PROJECT_NUMBER`, `GCP_WIF_PROVIDER`, `GCP_PLAN_SA`, `TF_STATE_BUCKET`.

- [ ] **Step 1: Write the failing redaction test**

Create `infra/scripts/redact-plan.test.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)

fail() { printf 'FAIL: %s\n--- got ---\n%s\n' "$1" "$2"; exit 1; }

got=$(printf 'email = "deploy@my-proj-123.iam.gserviceaccount.com"\nname = "projects/987654/locations/global"\n' | "$here/redact-plan.sh" my-proj-123 987654)
want=$(printf 'email = "deploy@***.iam.gserviceaccount.com"\nname = "projects/***/locations/global"\n')
[ "$got" = "$want" ] || fail "replaces every value" "$got"

got=$(printf 'a.b*c\n' | "$here/redact-plan.sh" 'a.b*c')
[ "$got" = "***" ] || fail "treats values literally, not as patterns" "$got"

got=$(printf 'unchanged\n' | "$here/redact-plan.sh" "")
[ "$got" = "unchanged" ] || fail "ignores empty values" "$got"

echo "redact-plan: all tests passed"
```

Run: `chmod +x infra/scripts/redact-plan.test.sh && infra/scripts/redact-plan.test.sh`
Expected: FAIL (`redact-plan.sh: No such file or directory`).

- [ ] **Step 2: Write the script**

Create `infra/scripts/redact-plan.sh`:

```bash
#!/usr/bin/env bash
# Replaces each argument with *** in stdin, literally. Used before posting a plan publicly.
set -euo pipefail

text=$(cat)
for value in "$@"; do
  [ -n "$value" ] || continue
  text=${text//"$value"/***}
done
printf '%s\n' "$text"
```

Run: `chmod +x infra/scripts/redact-plan.sh && infra/scripts/redact-plan.test.sh`
Expected: `redact-plan: all tests passed`.

- [ ] **Step 3: Write the workflow**

Create `.github/workflows/infra.yml`:

```yaml
name: Infra

on:
  pull_request:
    paths:
      - "infra/**"
      - ".github/workflows/infra.yml"
  push:
    branches: [main]
    paths:
      - "infra/**"
      - ".github/workflows/infra.yml"

permissions:
  contents: read

env:
  TF_VERSION: "1.16.5"
  TF_IN_AUTOMATION: "true"

jobs:
  check:
    name: Format, validate, test, scan
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7

      - uses: hashicorp/setup-terraform@v4
        with:
          terraform_version: ${{ env.TF_VERSION }}
          terraform_wrapper: false

      - name: Format
        run: terraform fmt -check -recursive infra

      - name: Module — validate and test
        working-directory: infra/modules/cloud-run-app
        run: |
          terraform init -backend=false -input=false
          terraform validate
          terraform test

      - name: Production — validate
        working-directory: infra/prod
        run: |
          terraform init -backend=false -input=false
          terraform validate

      - name: Redaction script test
        run: infra/scripts/redact-plan.test.sh

      - name: Trivy config scan
        uses: aquasecurity/trivy-action@ed142fd0673e97e23eac54620cfb913e5ce36c25 # v0.36.0
        with:
          scan-type: config
          scan-ref: infra
          severity: HIGH,CRITICAL
          exit-code: "1"

  # Plans need the GCP secrets, which exist only after the first apply and are not given to
  # Dependabot or fork PRs. Skip cleanly instead of failing.
  gate:
    name: Plan prerequisites
    if: github.event_name == 'pull_request' && github.event.pull_request.head.repo.full_name == github.repository
    runs-on: ubuntu-latest
    outputs:
      ready: ${{ steps.check.outputs.ready }}
    steps:
      - id: check
        env:
          GCP_PROJECT_ID: ${{ secrets.GCP_PROJECT_ID }}
        run: |
          if [ -n "$GCP_PROJECT_ID" ]; then
            echo "ready=true" >> "$GITHUB_OUTPUT"
          else
            echo "GCP secrets are not set; skipping the plan."
            echo "ready=false" >> "$GITHUB_OUTPUT"
          fi

  plan:
    name: Plan (read-only)
    needs: [check, gate]
    if: needs.gate.outputs.ready == 'true'
    runs-on: ubuntu-latest
    permissions:
      contents: read
      id-token: write
      pull-requests: write
    env:
      TF_VAR_project_id: ${{ secrets.GCP_PROJECT_ID }}
      TF_VAR_state_bucket: ${{ secrets.TF_STATE_BUCKET }}
    steps:
      - uses: actions/checkout@v7

      - uses: google-github-actions/auth@v3
        with:
          workload_identity_provider: ${{ secrets.GCP_WIF_PROVIDER }}
          service_account: ${{ secrets.GCP_PLAN_SA }}

      - uses: hashicorp/setup-terraform@v4
        with:
          terraform_version: ${{ env.TF_VERSION }}
          terraform_wrapper: false

      - name: Init
        working-directory: infra/prod
        run: terraform init -input=false -backend-config="bucket=${TF_VAR_state_bucket}"

      # -lock=false: taking the state lock writes to the bucket, and this account is read-only.
      - name: Plan
        id: plan
        working-directory: infra/prod
        env:
          GCP_PROJECT_NUMBER: ${{ secrets.GCP_PROJECT_NUMBER }}
        run: |
          set +e
          terraform plan -lock=false -input=false -no-color > plan.txt 2>&1
          echo "exit=$?" >> "$GITHUB_OUTPUT"
          set -e
          ../scripts/redact-plan.sh "$TF_VAR_project_id" "$GCP_PROJECT_NUMBER" < plan.txt > plan-redacted.txt
          cat plan-redacted.txt

      - name: Comment the plan on the PR
        uses: actions/github-script@v9
        with:
          script: |
            const fs = require('fs');
            const marker = '<!-- terraform-plan:prod -->';
            let plan = fs.readFileSync('infra/prod/plan-redacted.txt', 'utf8');
            if (plan.length > 60000) plan = plan.slice(0, 60000) + '\n… (truncated; see the job log)';
            const body = `${marker}\n### Terraform plan — prod\n\n\`\`\`\n${plan}\n\`\`\``;
            const { owner, repo } = context.repo;
            const issue_number = context.issue.number;
            const comments = await github.paginate(github.rest.issues.listComments, { owner, repo, issue_number });
            const previous = comments.find(c => c.user.type === 'Bot' && c.body.startsWith(marker));
            if (previous) {
              await github.rest.issues.updateComment({ owner, repo, comment_id: previous.id, body });
            } else {
              await github.rest.issues.createComment({ owner, repo, issue_number, body });
            }

      - name: Fail if the plan failed
        if: steps.plan.outputs.exit != '0'
        run: exit 1
```

- [ ] **Step 4: Add Terraform to Dependabot**

Append to `.github/dependabot.yml` (same indentation as the existing entries):

```yaml
  - package-ecosystem: terraform
    directories:
      - /infra/prod
      - /infra/modules/cloud-run-app
    schedule:
      interval: weekly
```

- [ ] **Step 5: Lint the workflow and run the scan locally**

Run:
```bash
docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.12 -color
docker run --rm -v "$PWD:/repo" -w /repo aquasec/trivy:0.75.0 config --severity HIGH,CRITICAL --exit-code 1 infra
```
Expected: `actionlint` prints nothing. Trivy reports no HIGH/CRITICAL findings. If Trivy reports a finding that is intended (for example the service being public), add `# trivy:ignore:<ID> <reason>` on the line above the resource; any other finding is fixed in the code. Re-run until it passes.

- [ ] **Step 6: Commit**

```bash
git add infra/scripts .github/workflows/infra.yml .github/dependabot.yml infra
git commit -m "ci: infra workflow — fmt, validate, module tests, Trivy, read-only plan on PRs

The plan runs only for same-repository PRs once the GCP secrets exist, and
the project ID and number are replaced with *** before the plan is posted.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Documentation

**Files:**
- Create: `infra/README.md`
- Modify: `README.md` (Hosting section), `docs/learning/hosting.md`

**Interfaces:**
- Consumes: resource and output names from Tasks 1–6.

- [ ] **Step 1: Write `infra/README.md`**

```markdown
# Infrastructure

Smoke Tracker's Google Cloud setup, as Terraform. Design:
[hosting infrastructure spec](../docs/specs/2026-10-13-hosting-infra-design.md).

## What is automatic and what is manual

| Piece | How | Why |
|---|---|---|
| Google project, billing account, €1 budget alert | Manual | Terraform needs a project with billing before it can run |
| State bucket | Manual, once | Terraform needs somewhere to keep its state before its first run |
| Neon project (Frankfurt) | Manual | No official Terraform provider; the community one can replace resources on upgrade, which for a database means losing data |
| Database connection string (the secret's value) | Manual, once | A value managed by Terraform is stored in its state in plain text |
| GitHub secrets and variable | Manual, once | Setting them from Terraform would need a GitHub admin token |
| Project APIs, image registry, secret (without value), service accounts and grants, GitHub login, Cloud Run service | Terraform | Reviewed in PRs, repeatable |

## Layout

- `modules/cloud-run-app/` — one app on Cloud Run; reusable. Tests: `terraform test` (mocked provider).
- `prod/` — Smoke Tracker's production: APIs, the module, the read-only `terraform-plan` account.
- `scripts/redact-plan.sh` — replaces the project ID and number before a plan is posted on a PR.

## Every change

1. Open a PR that changes `infra/`. CI checks, tests and scans it, then posts a plan comment.
2. Read the plan. Anything marked *destroy* or *replace* needs a reason.
3. Merge, then in Cloud Shell:

   ```bash
   cd ~/smoke-tracker && git pull
   cd infra/prod
   terraform plan    # should match the PR comment
   terraform apply
   ```

## First-time setup

Done once, in the Google Cloud console and Cloud Shell.

1. **State bucket.** Cloud Storage → Create: region `europe-west3`, Standard class, uniform access,
   public access prevention on, object versioning on.
2. **Terraform in Cloud Shell** (not preinstalled; `~/bin` survives between sessions):

   ```bash
   mkdir -p ~/bin && cd /tmp
   curl -fsSLO https://releases.hashicorp.com/terraform/1.16.5/terraform_1.16.5_linux_amd64.zip
   unzip -o terraform_1.16.5_linux_amd64.zip terraform -d ~/bin
   terraform version
   ```

3. **APIs Terraform needs to start:**

   ```bash
   gcloud services enable serviceusage.googleapis.com cloudresourcemanager.googleapis.com
   ```

4. **Configure and apply:**

   ```bash
   git clone https://github.com/locadani/smoke-tracker.git ~/smoke-tracker
   cd ~/smoke-tracker/infra/prod
   cp terraform.tfvars.example terraform.tfvars   # fill in project ID and bucket
   cp backend.hcl.example backend.hcl             # fill in bucket
   terraform init -backend-config=backend.hcl
   terraform plan    # the registry shows "will be imported", not "created"
   terraform apply
   ```

5. **Database secret.** Secret Manager → `smoke-tracker-database-url` → New version → paste the Neon
   *direct* connection string. Don't save it anywhere else.
6. **GitHub** (Settings → Secrets and variables → Actions), from `terraform output`:
   - Secrets: `GCP_PROJECT_ID`, `GCP_PROJECT_NUMBER` (`project_number`), `GCP_WIF_PROVIDER`
     (`workload_identity_provider`), `GCP_DEPLOY_SA` (`deploy_service_account`), `GCP_PLAN_SA`
     (`plan_service_account`), `TF_STATE_BUCKET`.
   - Variable: `GCP_REGION` = `europe-west3`.

   These are identifiers, not credentials; they are secrets so GitHub masks them in public logs.
7. **Connect the secret.** A PR sets `attach_database_url = true` in `prod/main.tf`; merge, then
   `terraform apply`.
8. **Check.** Open `terraform output service_url`: Google's sample page. A second
   `terraform plan` reports no changes.

The first plan comment on a PR may fail with "permission denied" for a few minutes after the
first apply, while permissions spread. Re-run the job.

## Changing the database password

Add a new version of `smoke-tracker-database-url`, **destroy** the old one (disabled versions still
count towards the 6 free ones), then redeploy: instances read the secret when they start.
```

- [ ] **Step 2: Update the README's Hosting section**

In `README.md`, replace the hosting diagram's Google Cloud subgraph label line and the table rows so they match the new names. Make these exact replacements:

- `sm[Secret Manager<br/>DATABASE_URL]` → `sm[Secret Manager<br/>smoke-tracker-database-url]`
- In the table, `Lets this repository's workflows act as narrowly-scoped service accounts (for example \`github-deploy\`)` → `Lets this repository's workflows act as narrowly-scoped service accounts (\`smoke-tracker-deploy\` for deploys, \`terraform-plan\` for read-only plans)`
- `Runs the app as the \`smoke-run\` service account.` → `Runs the app as the \`smoke-tracker-run\` service account.`
- `Holds the database connection string; only \`smoke-run\` can read it | \`DATABASE_URL\` |` → `Holds the database connection string; only \`smoke-tracker-run\` can read it. The app sees it as \`DATABASE_URL\` | \`smoke-tracker-database-url\` |`
- `None. Only non-secret IDs (project, region, login provider, account names), as repository variables` → `None. Project and account identifiers are stored as GitHub secrets only so they are masked in public logs`

Then, after the sentence `No secret is ever stored in this repository, in a Docker image, or in GitHub.`, add:

```markdown

The Google Cloud side is Terraform: see [`infra/`](infra/README.md) for what is automatic, what is
manual and why, and how changes are applied.
```

- [ ] **Step 3: Add a Terraform section to the learning note**

In `docs/learning/hosting.md`, insert before `## Where to find this without help`:

```markdown
## Terraform

- **Plan vs apply.** `plan` compares the code with reality and lists what would change; `apply`
  makes the change. Here CI only plans (read-only account); applying is done by hand.
- **State.** After each apply Terraform records what it manages in a state file, kept in a
  versioned bucket. Anything Terraform manages ends up in it, which is why secret values stay
  out of Terraform.
- **Import.** Something created by hand can be adopted with an `import` block instead of being
  recreated. The plan says "will be imported".
- **`ignore_changes`.** When something else owns an attribute (the deploy workflow owns the
  image), Terraform must ignore it, or every apply would undo the deploy.
- **Modules.** `modules/cloud-run-app` describes one app; `prod/` uses it. A second app reuses the
  module with different inputs.
- **Tests without a cloud account.** `terraform test` with `mock_provider` checks the
  configuration (names, permissions, settings) offline, in CI.
- **Least privilege.** Each grant is on one resource where possible (one secret, one registry,
  one service), not on the whole project.
```

And add to the `## Where to find this without help` list:

```markdown
- Terraform tests and mocks: <https://developer.hashicorp.com/terraform/language/tests/mocking>
- Import blocks: <https://developer.hashicorp.com/terraform/language/import>
- Google provider docs: <https://registry.terraform.io/providers/hashicorp/google/latest/docs>
```

- [ ] **Step 4: Check the docs**

Run: `grep -rn -E "smoke-run|github-deploy" README.md docs/learning/hosting.md infra/README.md`
Expected: no matches (the old names are gone).

- [ ] **Step 5: Commit**

```bash
git add infra/README.md README.md docs/learning/hosting.md
git commit -m "docs: infra README (manual vs Terraform, setup, routine) and Terraform notes

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Decision log (user approval required)

**Files:**
- Modify: `docs/DECISIONS.md`

**Interfaces:**
- Consumes: the spec's decisions.

- [ ] **Step 1: Draft the entries and show them to the user**

The milestone 3 branch adds decisions #56–#73, not yet on `main`. Use `#74` onwards so the numbers don't clash. Draft, in the table's existing format (`| # | Date | Area | Decision | Alternatives | Why |`):

```markdown
| 74 | 2026-10-13 | Hosting | **Google Cloud Run** (europe-west3) + **Neon** Postgres (Frankfurt), free tiers; replaces #39 | Oracle Always Free VM (kept as fallback); Render; Koyeb; GCP e2-micro | Live quickly with nothing to maintain; a Rust container cold-starts in about a second; cost ≈ €0.00–0.01/month |
| 75 | 2026-10-13 | Database | **Postgres (Neon) in production**, SQLite for local development and tests; replaces #38 | SQLite on a VM with Litestream | Cloud Run has no persistent disk; the Postgres backend already passes the shared contract tests |
| 76 | 2026-10-13 | Infra | Google Cloud side as **Terraform** in `infra/` (module + `prod/`), state in a versioned GCS bucket | Console clicks; Cloud Build deploys; separate infra repo | Reviewed in PRs, repeatable for a second app, part of the portfolio |
| 77 | 2026-10-13 | Infra | **Plan in CI with a read-only account, apply by hand** in Cloud Shell | Apply from CI | No account with write access is usable from CI |
| 78 | 2026-10-13 | Infra | Neon, the secret value, the state bucket and GitHub settings stay **manual** | Neon via community provider; secrets in Terraform | No official Neon provider; values in Terraform end up in its state |
| 79 | 2026-10-13 | Security | Keyless deploys with **Workload Identity Federation**, limited by repository ID and to `deploy.yml` on `main`; secrets only in Secret Manager | Service account key in GitHub; GitHub secret passed as a plain env var | No long-lived key; plain env vars are visible in the Cloud Run console |
```

Ask the user to approve the wording. Do not commit before they approve.

- [ ] **Step 2: Append the approved entries and commit**

Append the approved rows at the end of the table in `docs/DECISIONS.md`.

```bash
git add docs/DECISIONS.md
git commit -m "docs(decisions): hosting on Cloud Run + Neon, Terraform, manual pieces, keyless deploys

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## After this plan (the user, in the console and Cloud Shell)

Follow `infra/README.md` → First-time setup. Then (spec §8), after milestone 3 merges: `deploy.yml`, the app's settings, the daily backup, and the `create-resident` job, each in its own PR.
