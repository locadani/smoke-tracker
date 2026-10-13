# Hosting Infrastructure (Terraform) — Design Spec

- **Date:** 2026-10-13
- **Status:** Approved (2026-10-13, from a one-screen summary), revised after an independent review against official docs
- **Extends:** [design spec](2026-10-13-smoke-tracker-design.md) §13.2 (hosting was deferred)

---

## 1. Purpose

Host Smoke Tracker on **Google Cloud Run** with its data in **Neon** Postgres, and describe the
Google Cloud side as **Terraform** code in this repository.

**Goals**

1. **Learning:** Terraform, Google Cloud permissions, keyless CI login.
2. **Portfolio:** infrastructure that is reviewed in PRs and checked in CI, next to the app.
3. **Repeatability:** the whole Google setup can be recreated with Terraform, and a second app
   can reuse the same module.

**Constraints**

- **Cost ≈ €0.** Everything runs inside free tiers except outgoing traffic from Frankfurt (the
  free allowance covers North America only) and the small state and backup buckets (the free storage tier
  is US-only). Expected bill: €0.00–0.01 a month. A €1 budget alert (created by `infra/bootstrap`) warns; it does not cap.
- No secret in the repository, in Terraform state, in a Docker image, or in GitHub.
- CI/CD tracked and triggered by GitHub.
- The development machine is a work PC: nothing is installed locally. Terraform runs in
  **Google Cloud Shell** (browser).

**Non-goals:** Terraform for Neon; Terraform applied automatically by CI; a separate
infrastructure repository; running `create-resident` as a Cloud Run job (§8).

---

## 2. Manual vs Terraform

| Piece | How | Why |
|---|---|---|
| Google project, billing account | Manual (done) | Terraform needs a project with billing before it can run |
| €1 budget alert | Terraform (`bootstrap/`), applied by hand | Billing permissions stay out of CI; a hand-made budget is deleted so alerts don't arrive twice |
| State bucket | Terraform (`bootstrap/`), first run with a local state | Terraform needs somewhere to keep its state, so this small configuration creates the bucket, then moves its own state into it |
| Neon project (Frankfurt, `aws-eu-central-1`) | Manual (done) | Neon has no official Terraform provider; the community one warns that upgrades can replace resources, which for a database means losing data |
| `DATABASE_URL` secret **value** | Manual, once | A value managed by Terraform is stored in its state in plain text |
| GitHub secrets and variable | Manual, once, copied from Terraform outputs | Setting them from Terraform would need a GitHub admin token |
| Project APIs | Terraform | The 5 enabled by hand are adopted (enabling is idempotent); the others it needs are added (§4.0) |
| Artifact Registry repository + cleanup policies | Terraform | Already created by hand; adopted with an `import` block, and its incomplete cleanup policy is corrected (§4.3) |
| Secret Manager secret `DATABASE_URL` (the container, no value) | Terraform | |
| Service accounts and their permissions | Terraform | |
| Workload Identity Federation pool and provider | Terraform | |
| Cloud Run service | Terraform | Terraform owns its settings; the deploy workflow owns which image runs |
| App settings (base URL, client IP header, log level) | Terraform (`prod/main.tf`) | Reviewed in PRs |
| Nightly backup (bucket, job, schedule) | Terraform (module `backup.tf`) | Code like the rest (§8) |
| First resident | Manual, once, from Cloud Shell | The command asks for the password interactively (§8) |

`infra/README.md` repeats this table.

---

## 3. Layout

```
infra/
├─ README.md               manual vs Terraform, first-time setup, routine
├─ bootstrap/              the state bucket and the €1 budget alert (local state first, then migrated)
├─ scripts/                redact-plan.sh (and its test): hides IDs before a plan is posted on a PR
├─ modules/
│  └─ cloud-run-app/       one app on Cloud Run (reusable)
│     ├─ main.tf           Artifact Registry, secret, service accounts and their grants
│     ├─ service.tf        Cloud Run service
│     ├─ backup.tf         nightly backup: bucket, job, schedule, grants
│     ├─ github-login.tf   Workload Identity Federation, limited to one repository
│     ├─ variables.tf
│     ├─ outputs.tf
│     └─ tests/            `terraform test` with a mocked provider (no Google account needed)
└─ prod/                   Smoke Tracker's production
   ├─ main.tf              uses the module
   ├─ project.tf           project APIs
   ├─ imports.tf           adopts the hand-made Artifact Registry repository
   ├─ ci.tf                the read-only `terraform-plan` account
   ├─ backend.tf           GCS backend (bucket name supplied at init, see §3.1)
   ├─ variables.tf
   ├─ versions.tf          pinned Terraform and provider versions
   ├─ terraform.tfvars.example
   └─ backend.hcl.example
```

- The module knows nothing about Smoke Tracker; `prod/` passes in the app name, region,
  repository ID and settings.
- **Terraform**, not OpenTofu: the most widely recognised tool, and its licence is fine for
  personal use. It is **not** preinstalled in Cloud Shell (removed June 2026): the pinned version
  is downloaded once into `~/bin`, which persists between Cloud Shell sessions.
- Exact Terraform and `hashicorp/google` versions are pinned in `versions.tf`. The plan chooses
  them from current releases, and checks that Dependabot supports the chosen Terraform version.
- `.terraform.lock.hcl` is committed for `prod/` and `bootstrap/`.
- Resource names come from the app name, so a second app in the same project doesn't clash:
  `smoke-tracker-run`, `smoke-tracker-deploy`, secret `smoke-tracker-database-url`.

### 3.1 Values kept out of the repository

The project ID and state bucket name are not secret, but are kept out of the public repository,
like the GitHub secrets (§6):

- `prod/` and `bootstrap/` each have a `terraform.tfvars` and a `backend.hcl`, **gitignored**; `.example` files are
  committed.
- In Cloud Shell, the user creates them once from the examples.
- In CI they come from GitHub **secrets** (§6), which GitHub masks in logs automatically.

This is tidiness, not protection: the project number also appears in the service's default URL,
and access is controlled by permissions (§4). The GitHub repository ID (`1416787003`) is public
and is committed in `prod/`.

---

## 4. Resources and permissions

### 4.0 Project APIs

Enabled with `google_project_service`, `disable_on_destroy = false`: `run`, `artifactregistry`,
`secretmanager`, `iam`, `iamcredentials`, `sts`, `cloudresourcemanager`, `serviceusage`,
`storage`. `serviceusage` and `cloudresourcemanager` must already be on before the first `apply`
(Terraform uses them to enable the rest); the first-time setup checks this (§7). The Container
Scanning API is **not** enabled (it is billed per image).

### 4.1 Service accounts

| Account | Used by | Permissions (narrowest scope available) |
|---|---|---|
| `smoke-tracker-run` | the app on Cloud Run | `secretmanager.secretAccessor` on the `smoke-tracker-database-url` secret only |
| `smoke-tracker-deploy` | the deploy workflow only (§4.2) | `artifactregistry.writer` on the app's repository; `run.developer` on the app's service; `iam.serviceAccountUser` on `smoke-tracker-run` |
| `terraform-plan` | workflows of this repository (§4.2) | `roles/viewer` and `roles/iam.securityReviewer` on the project; `storage.objectViewer` on the state bucket. Cannot change resources or read secret values |

- `run.developer` cannot change who may call the service, so the deploy workflow must never pass
  `--allow-unauthenticated`; public access is set by Terraform (§4.5).
- `roles/viewer` lets the plan account read logs and plain service settings. That is acceptable
  because no secret is ever a plain setting.
- The user runs `apply` as themselves (project owner) in Cloud Shell. No account with write
  access to the infrastructure is usable from CI.

### 4.2 GitHub login (Workload Identity Federation)

- Pool `github`, OIDC provider `github-actions`, issuer
  `https://token.actions.githubusercontent.com`.
- Attribute mapping includes `google.subject = assertion.sub`,
  `attribute.repository_id = assertion.repository_id` and the workflow file and branch taken from
  `assertion.job_workflow_ref`.
- Provider condition: `assertion.repository_id == '1416787003'`. The ID is used rather than the
  name because a name can be re-registered by someone else.
- `smoke-tracker-deploy` can be impersonated only by `.github/workflows/deploy.yml` running on
  `main` (binding on `attribute.job_workflow_ref`).
- `terraform-plan` can be impersonated by any workflow of the repository. A binding can match only
  one exact attribute value, so "`infra.yml` on any branch" can't be expressed; the account is
  read-only, so this is acceptable.
- If a second app later shares this Google project, the pool moves out of the module into
  `prod/`. Until then it stays in the module.

### 4.3 Artifact Registry

Repository `smoke-tracker`, Docker, `europe-west3`, adopted from the hand-made repository with an
`import` block. Cleanup needs **two** policies, because a keep policy alone deletes nothing:

- **delete:** any image older than 1 day;
- **keep:** the 3 most recent versions (protected from the delete policy).

Dry run off. The keep-only policy set by hand is replaced.

### 4.4 Secret Manager

Secret `smoke-tracker-database-url` (the app still reads it as the `DATABASE_URL` environment
variable), user-managed replication in `europe-west3`, **no version** in Terraform,
`deletion_protection = true`. When the password changes, the old version is
**destroyed**, not just disabled: disabled versions still count towards the 6 free active
versions.

### 4.5 Cloud Run service

- Name `smoke-tracker`, region `europe-west3`.
- Public through `invoker_iam_disabled = true` (Google's recommended way), not an `allUsers`
  grant.
- Scaling 0 to 1 instances; CPU allocated only during requests; 1 vCPU, 512 MiB memory (less
  than 512 MiB would force the first-generation runtime).
- Runs as `smoke-tracker-run`.
- Initial image: Google's sample `us-docker.pkg.dev/cloudrun/container/hello`.
- **`DATABASE_URL` is attached only when the secret has a value.** Cloud Run refuses to start an
  instance whose secret can't be read, so a module input (`attach_database_url`, default
  `false`) controls it. It is set in `prod/main.tf` (committed), so CI and Cloud Shell plan the
  same thing. The first `apply` runs the sample image without it; after the value is added, a
  small PR sets it to `true` (§7). The service `depends_on` the runtime account's secret grant.
- `lifecycle { ignore_changes = [template[0].containers[0].image, client, client_version] }`:
  the deploy workflow changes the image, and Terraform must not roll it back. The deploy
  workflow skips its default labels (§8), so deploys don't show up as drift.
- `deletion_protection = true`.
- The app's settings are the module's `env` input, set in `prod/main.tf` (§8).

### 4.6 Outputs

Service URL, project number, Workload Identity provider name, deploy and plan account emails,
image repository URL. These are copied into GitHub (§6).

---

## 5. CI: `infra.yml`

Trigger: pull requests and pushes to `main` that change `infra/**` or `infra.yml`.

| Job | Steps |
|---|---|
| `check` | `terraform fmt -check -recursive`; `terraform init -backend=false`, `terraform validate` and `terraform test` for the module and for `bootstrap/`; `validate` for `prod/`; the redaction script's test; Trivy config scan of `infra/` |
| `plan` | Pull requests from this repository only (`head.repo.full_name == github.repository`). Logs in as `terraform-plan` through Workload Identity Federation; `terraform init` with the backend config from secrets; `terraform plan -lock=false`; posts the plan as a PR comment, updating the previous one |

- `plan` uses `-lock=false` because taking the lock writes to the bucket, and the plan account
  is read-only.
- Workflow permissions: `contents: read`, `id-token: write` (plan only),
  `pull-requests: write` (plan only).
- `plan` is skipped when the `GCP_PROJECT_ID` secret is empty (before the first `apply`, and on
  Dependabot PRs, which don't get repository secrets). Right after the
  first `apply`, permissions can take a few minutes to spread, so the first plan may fail with
  "permission denied"; re-running it fixes that.
- Third-party actions that aren't from GitHub or Google (Trivy) are pinned to a commit SHA, not a
  tag, because tags can be moved.
- Trivy findings that are intended (the service is public on purpose) are suppressed one by one,
  each with its reason, never with a blanket rule.
- Dependabot gains a `terraform` entry (weekly) for `infra/prod` and `infra/modules/cloud-run-app`.

Applying stays manual (§7).

### 5.1 Keeping IDs out of public logs

The repository is public, so CI logs and PR comments are public. Terraform's plan prints the
project ID and number on most lines.

- The IDs are stored as GitHub **secrets**, not variables: GitHub masks secrets in logs
  automatically, while variable values are printed in each step's header.
- Masking covers logs only. Before the plan is posted as a PR comment, a small tested script
  (`infra/scripts/redact-plan.sh`) replaces the project ID, project number and state bucket name with `***`.
- Nothing else is published: no job summary, and the binary plan file is never uploaded as an
  artifact.

---

## 6. GitHub secrets and variable

- **Secrets** (identifiers, not credentials; stored as secrets only so GitHub masks them):
  `GCP_PROJECT_ID`, `GCP_PROJECT_NUMBER`, `GCP_WIF_PROVIDER`, `GCP_DEPLOY_SA`, `GCP_PLAN_SA`,
  `TF_STATE_BUCKET`.
- **Variable:** `GCP_REGION`.

Set by hand from the Terraform outputs.

---

## 7. Running it

**First time**

1. In Cloud Shell, download the pinned Terraform into `~/bin`, check that the Service Usage,
   Cloud Resource Manager and Cloud Storage APIs are enabled, and clone the repository.
2. Run `infra/bootstrap` in Cloud Shell: it creates the state bucket (`europe-west3`, standard
   class, uniform access, public access prevention, object versioning, the current state plus 9 older versions kept) with a local state, then moves its own state into the bucket; the local copy is deleted only after
   the bucket is checked to hold it. It also creates the €1 budget alert.
3. In Cloud Shell:
   1. create `terraform.tfvars` and `backend.hcl` in `infra/prod` from the examples;
   2. `terraform init`, `terraform plan` (the registry shows as *import*, not *create*),
      `terraform apply`.
4. Add the Neon connection string as the first version of the `smoke-tracker-database-url`
   secret (console).
5. Set the GitHub secrets and variable (§6).
6. A small PR sets `attach_database_url = true` in `prod/main.tf`; after merging,
   `terraform apply`.
7. Open the service URL: Google's sample page appears.
8. Re-run the latest Deploy run in GitHub (Actions → Deploy → Re-run all jobs): the app replaces
   the sample page.
9. Create the first resident from Cloud Shell (§8).

**Routine:** PR → read the plan comment → merge → in Cloud Shell `git pull`, `terraform plan`
(should match the comment), `terraform apply`.

**Changing the database password:** add a new secret version, destroy the old one, then
redeploy (instances read the secret at startup).

---

## 8. Settings, deploys, backups, first resident

Built after the first apply, in the go-live plan (`docs/plans/infra-go-live.md`).

- **Settings.** The module's `env` input; `prod/` sets `APP_BASE_URL` (built from the project
  number), `CLIENT_IP_HEADER=X-Forwarded-For` and `RUST_LOG=info`.
- **`deploy.yml`.** Runs on push to `main` only, never `workflow_call`, `pull_request_target` or
  `workflow_dispatch`, because the deploy account trusts `deploy.yml` on `main`. It ignores
  changes to docs and `infra/`, and skips until the GitHub secrets exist. It builds the image
  **once** with a neutral tag, scans that image with Trivy, and only then logs in, so the scanned
  image is the deployed image. A guard refuses to deploy while `DATABASE_URL` is not attached to
  the service (without it the image would fall back to a throwaway SQLite file). It then pushes
  the image tagged with the commit SHA to Artifact Registry and runs
  `gcloud run services update --image`; the default labels are skipped, so deploys are not drift.
  This replaces the GHCR push in the original spec's `release.yml` (§13.1), which is deleted.
- **Database connection.** Neon's **direct** connection (the app keeps its own pool; migrations
  need a direct connection). Neon suspends after 5 idle minutes, so the pool's idle timeout is
  under 5 minutes and connections are tested before use. The string uses `sslmode=require`:
  encrypted, but sqlx does not verify the certificate; `verify-full` would need
  `sslrootcert=system` for `pg_dump`, which sqlx would read as a file path. Neon's
  `channel_binding=require` is ignored by sqlx, with a warning in the log.
- **Backup.** Cloud Scheduler starts the Cloud Run job `smoke-tracker-backup` daily at 03:00
  (Europe/Berlin). It runs `pg_dump --format=custom` into the bucket `<project>-smoke-tracker-backups`
  (`europe-west3`), as `smoke-tracker-<UTC time>.dump`, written first as `.dump.partial` and
  renamed on success. Dumps are deleted after 30 days. The job exists only once
  `attach_database_url` is `true`. The bucket is in the EU, so outside the free tier; a few dumps
  cost fractions of a cent. Neon's free plan keeps only 6 hours of history, which is why this
  exists. It replaces Litestream → R2 in the original spec (§13.2). Restore with
  `pg_restore --clean --if-exists --no-owner --single-transaction`, run in a `postgres:18-alpine`
  container so the client matches the dump; it overwrites live data (`infra/README.md`).
- **First resident.** Run once from Cloud Shell: `docker run -it` of the deployed image with
  `create-resident`, passing `DATABASE_URL` from Secret Manager to that one process. A Cloud Run
  job was considered and dropped: the command asks for the password on a terminal, and a job has
  none, so the password would have to be an argument or a secret, which leaks it into the job's
  configuration or logs.

---

## 9. Verification

- CI: `check` passes; `plan` posts a comment on a test PR, with the project ID and number shown
  as `***`.
- After the first-time setup: `terraform plan` reports **no changes**; the service URL shows the
  sample page; after go-live, the app's sign-in page, where the first resident can sign in.
- The `import` adopts the existing Artifact Registry repository without replacing it.
- The Artifact Registry repository shows both cleanup policies, dry run off.

---

## 10. Documentation

- `infra/README.md`: §2 table, first-time setup, routine, password change.
- `README.md`: Terraform in the hosting diagram; link to `infra/`.
- `docs/learning/hosting.md`: state, plan vs apply, imports, why some pieces stay manual.
- `docs/DECISIONS.md`: entries for Cloud Run + Neon (replacing #38 and #39), Terraform, state in
  GCS, Neon manual, apply from Cloud Shell. Numbered after milestone 3's #56–#75. Wording shown
  to the user before committing.
