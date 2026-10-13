# Hosting Infrastructure (Terraform) — Design Spec

- **Date:** 2026-10-13
- **Status:** Draft, for review (revised after an independent review against official docs)
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
  free allowance covers North America only) and the few-KB state bucket (the free storage tier is
  US-only). Expected bill: €0.00–0.01 a month. A €1 budget alert warns; it does not cap.
- No secret in the repository, in Terraform state, in a Docker image, or in GitHub.
- CI/CD tracked and triggered by GitHub.
- The development machine is a work PC: nothing is installed locally. Terraform runs in
  **Google Cloud Shell** (browser).

**Non-goals:** Terraform for Neon; Terraform applied automatically by CI; a separate
infrastructure repository; the deploy workflow, the daily backup and running admin commands in
production (all follow milestone 3, §8).

---

## 2. Manual vs Terraform

| Piece | How | Why |
|---|---|---|
| Google project, billing account, €1 budget alert | Manual (done) | Terraform needs a project with billing before it can run |
| State bucket | Manual, once | Terraform needs somewhere to keep its state before its first run |
| Neon project (Frankfurt, `aws-eu-central-1`) | Manual (done) | Neon has no official Terraform provider; the community one warns that upgrades can replace resources, which for a database means losing data |
| `DATABASE_URL` secret **value** | Manual, once | A value managed by Terraform is stored in its state in plain text |
| GitHub repository variables | Manual, once, copied from Terraform outputs | Setting them from Terraform would need a GitHub admin token |
| Project APIs | Terraform | The 5 enabled by hand are adopted (enabling is idempotent); the others it needs are added (§4.0) |
| Artifact Registry repository + cleanup policies | Terraform | Already created by hand; adopted with an `import` block, and its incomplete cleanup policy is corrected (§4.3) |
| Secret Manager secret `DATABASE_URL` (the container, no value) | Terraform | |
| Service accounts and their permissions | Terraform | |
| Workload Identity Federation pool and provider | Terraform | |
| Cloud Run service | Terraform | Terraform owns its settings; the deploy workflow owns which image runs |

`infra/README.md` repeats this table.

---

## 3. Layout

```
infra/
├─ README.md               manual vs Terraform, first-time setup, routine
├─ modules/
│  └─ cloud-run-app/       one app on Cloud Run (reusable)
│     ├─ main.tf           Cloud Run service, Artifact Registry, secret, service accounts
│     ├─ github-login.tf   Workload Identity Federation, limited to one repository
│     ├─ variables.tf
│     └─ outputs.tf
└─ prod/                   Smoke Tracker's production
   ├─ main.tf              uses the module
   ├─ project.tf           project APIs
   ├─ imports.tf           adopts the hand-made Artifact Registry repository
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
- `.terraform.lock.hcl` is committed.

### 3.1 Values kept out of the repository

The project ID and state bucket name are not secret, but are kept out of the public repository,
like the GitHub variables (§6):

- `prod/terraform.tfvars` and `prod/backend.hcl` are **gitignored**; `.example` files are
  committed.
- In Cloud Shell, the user creates them once from the examples.
- In CI they are passed from GitHub variables (`-var`, `-backend-config`), and masked in logs
  (§5.1).

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
| `smoke-run` | the app on Cloud Run | `secretmanager.secretAccessor` on the `DATABASE_URL` secret only |
| `github-deploy` | the deploy workflow only (§4.2) | `artifactregistry.writer` on the app's repository; `run.developer` on the app's service; `iam.serviceAccountUser` on `smoke-run` |
| `terraform-plan` | the infra workflow only (§4.2) | `roles/viewer` and `roles/iam.securityReviewer` on the project; `storage.objectViewer` on the state bucket. Cannot change resources or read secret values |

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
- `github-deploy` can be impersonated only by `.github/workflows/deploy.yml` running on `main`.
- `terraform-plan` can be impersonated only by `.github/workflows/infra.yml`, on any branch.
- The exact mapping expressions are chosen in the plan and checked against the GitHub OIDC claim
  format.
- If a second app later shares this Google project, the pool moves out of the module into
  `prod/`. Until then it stays in the module.

### 4.3 Artifact Registry

Repository `smoke-tracker`, Docker, `europe-west3`, adopted from the hand-made repository with an
`import` block. Cleanup needs **two** policies, because a keep policy alone deletes nothing:

- **delete:** any image older than 1 day;
- **keep:** the 3 most recent versions (protected from the delete policy).

Dry run off. The keep-only policy set by hand is replaced.

### 4.4 Secret Manager

Secret `DATABASE_URL`, user-managed replication in `europe-west3`, **no version** in Terraform.
`lifecycle { prevent_destroy = true }`. When the password changes, the old version is
**destroyed**, not just disabled: disabled versions still count towards the 6 free active
versions.

### 4.5 Cloud Run service

- Name `smoke-tracker`, region `europe-west3`.
- Public through `invoker_iam_disabled = true` (Google's recommended way), not an `allUsers`
  grant.
- Scaling 0 to 1 instances; CPU allocated only during requests; 256 MiB memory.
- Runs as `smoke-run`.
- Initial image: Google's sample `us-docker.pkg.dev/cloudrun/container/hello`.
- **`DATABASE_URL` is attached only when the secret has a value.** Cloud Run refuses to start an
  instance whose secret can't be read, so a module input (`attach_database_url`, default
  `false`) controls it. The first `apply` runs the sample image without it; after the value is
  added, it is set to `true` (§7). The service `depends_on` the `smoke-run` secret grant.
- `lifecycle { ignore_changes = [template[0].containers[0].image, client, client_version] }`:
  the deploy workflow changes the image, and Terraform must not roll it back. The deploy
  workflow skips its default labels (§8), so deploys don't show up as drift.
- `deletion_protection = true`.
- The app's other settings (for example `CLIENT_IP_HEADER`) are added after milestone 3 (§8).

### 4.6 Outputs

Service URL, project ID, region, Workload Identity provider name, `github-deploy` email,
`terraform-plan` email. These are copied into GitHub variables (§6).

---

## 5. CI: `infra.yml`

Trigger: pull requests and pushes to `main` that change `infra/**` or `infra.yml`.

| Job | Steps |
|---|---|
| `check` | `terraform fmt -check -recursive`; `terraform init -backend=false` and `terraform validate` for `prod/` and the module; Trivy config scan of `infra/` |
| `plan` | Pull requests from this repository only (`head.repo.full_name == github.repository`). Logs in as `terraform-plan` through Workload Identity Federation; `terraform init` with the backend config from variables; `terraform plan -lock=false`; posts the plan as a PR comment, updating the previous one |

- `plan` uses `-lock=false` because taking the lock writes to the bucket, and the plan account
  is read-only.
- Workflow permissions: `contents: read`, `id-token: write` (plan only),
  `pull-requests: write` (plan only).
- `plan` is skipped when `GCP_PROJECT_ID` is empty (before the first `apply`). Right after the
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

- The workflow registers the project ID and project number with `::add-mask::`, so GitHub
  replaces them with `***` in logs.
- Masking covers logs only. Before the plan is posted as a PR comment, the same values are
  replaced with `***` in the comment text.
- Nothing else is published: no job summary, and the binary plan file is never uploaded as an
  artifact.

---

## 6. GitHub repository variables

`GCP_PROJECT_ID`, `GCP_PROJECT_NUMBER`, `GCP_REGION`, `GCP_WIF_PROVIDER`, `GCP_DEPLOY_SA`,
`GCP_PLAN_SA`, `TF_STATE_BUCKET`. None is a secret. They are set by hand from the Terraform
outputs and the project dashboard.

---

## 7. Running it

**First time**

1. Create the state bucket: Cloud Storage, `europe-west3`, standard class, uniform access,
   public access prevention on, object versioning on.
2. In Cloud Shell:
   1. download the pinned Terraform into `~/bin`;
   2. check that the Service Usage and Cloud Resource Manager APIs are enabled;
   3. clone the repository; create `terraform.tfvars` and `backend.hcl` from the examples;
   4. `terraform init`, `terraform plan` (the registry shows as *import*, not *create*),
      `terraform apply`.
3. Add the Neon connection string as the first version of the `DATABASE_URL` secret (console).
4. Set `attach_database_url = true` in `terraform.tfvars`; `terraform apply`.
5. Set the GitHub variables (§6).
6. Open the service URL: Google's sample page appears.

**Routine:** PR → read the plan comment → merge → in Cloud Shell `git pull`, `terraform plan`
(should match the comment), `terraform apply`.

**Changing the database password:** add a new secret version, destroy the old one, then
redeploy (instances read the secret at startup).

---

## 8. After milestone 3

Separate PRs, not part of this spec's implementation:

- `deploy.yml`: on push to `main`, build the `linux/amd64` image, push it to Artifact Registry and
  deploy it to Cloud Run as `github-deploy`, with the action's default labels skipped
  (`skip_default_labels: true`). This replaces the GHCR push in the original spec's
  `release.yml` (§13.1); that spec gets a short "superseded" note, with the user's approval.
- The app's settings as Cloud Run environment variables in `prod/`.
- Database connection: Neon's **direct** connection (the app keeps its own pool; migrations need
  a direct connection). Neon suspends after 5 idle minutes, so the pool's idle timeout is under
  5 minutes and connections are tested before use.
- Admin commands such as `create-resident` run as a one-off Cloud Run job with the same image and
  secret.
- Daily backup of the Neon database (Neon's free plan keeps 6 hours of history). This replaces
  Litestream → R2 in the original spec (§13.2), with the same "superseded" note.

---

## 9. Verification

- CI: `check` passes; `plan` posts a comment on a test PR, with the project ID and number shown
  as `***`.
- After the first-time setup: `terraform plan` reports **no changes**; the service URL shows the
  sample page.
- The `import` adopts the existing Artifact Registry repository without replacing it.
- The Artifact Registry repository shows both cleanup policies, dry run off.

---

## 10. Documentation

- `infra/README.md`: §2 table, first-time setup, routine, password change.
- `README.md`: Terraform in the hosting diagram; link to `infra/`.
- `docs/learning/hosting.md`: state, plan vs apply, imports, why some pieces stay manual.
- `docs/DECISIONS.md`: entries for Cloud Run + Neon (replacing #38 and #39), Terraform, state in
  GCS, Neon manual, apply from Cloud Shell. Numbered after milestone 3's #56–#73. Wording shown
  to the user before committing.
