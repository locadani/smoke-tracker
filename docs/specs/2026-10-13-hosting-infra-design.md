# Hosting Infrastructure (Terraform) — Design Spec

- **Date:** 2026-10-13
- **Status:** Draft, for review
- **Extends:** [design spec](2026-10-13-smoke-tracker-design.md) §13.2 (hosting was deferred)

---

## 1. Purpose

Host Smoke Tracker on **Google Cloud Run** with its data in **Neon** Postgres, and describe the
Google Cloud side as **Terraform** code in this repository.

**Goals**

1. **Learning:** Terraform, Google Cloud permissions, keyless CI login.
2. **Portfolio:** infrastructure that is reviewed in PRs and checked in CI, next to the app.
3. **Repeatability:** the whole Google setup can be recreated with one `apply`, and a second app
   can reuse the same module.

**Constraints**

- Zero cost (free tiers; a few KB of state storage is the only non-free-tier item, ≈ €0.00).
- No secret in the repository, in Terraform state, in a Docker image, or in GitHub.
- CI/CD tracked and triggered by GitHub.
- The development machine is a work PC: nothing is installed locally. Terraform is run in
  **Google Cloud Shell** (browser).

**Non-goals:** Terraform for Neon; Terraform applied automatically by CI; a separate
infrastructure repository; the deploy workflow and the daily backup (both follow milestone 3,
§8).

---

## 2. Manual vs Terraform

| Piece | How | Why |
|---|---|---|
| Google project, billing account, €1 budget alert | Manual (done) | Terraform needs a project with billing before it can run |
| State bucket | Manual, once | Terraform needs somewhere to keep its state before its first run |
| Neon project (Frankfurt, `aws-eu-central-1`) | Manual (done) | Neon has no official Terraform provider; the community one warns that upgrades can replace resources, which for a database means losing data |
| `DATABASE_URL` secret **value** | Manual, once | A value managed by Terraform is stored in its state in plain text |
| GitHub repository variables | Manual, once, copied from Terraform outputs | Setting them from Terraform would need a GitHub admin token |
| Project APIs | Terraform | The 5 already enabled are adopted (enabling is idempotent) |
| Artifact Registry repository + cleanup policy | Terraform | Already created by hand; adopted with an `import` block |
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
- **Terraform**, not OpenTofu: preinstalled in Cloud Shell and widely recognised. Exact
  Terraform and `hashicorp/google` versions are pinned in `versions.tf`; the plan chooses them
  from current releases.
- `.terraform.lock.hcl` is committed.

### 3.1 Values kept out of the repository

The project ID and state bucket name are not secret, but are kept out of the public repository,
like the GitHub variables (§6):

- `prod/terraform.tfvars` and `prod/backend.hcl` are **gitignored**; `.example` files are
  committed.
- In Cloud Shell, the user creates them once from the examples.
- In CI they are passed from GitHub variables (`-var`, `-backend-config`).

The GitHub repository ID (`1416787003`) is public and is committed in `prod/`.

---

## 4. Resources and permissions

### 4.1 Service accounts

| Account | Used by | Permissions (narrowest scope available) |
|---|---|---|
| `smoke-run` | the app on Cloud Run | `secretmanager.secretAccessor` on the `DATABASE_URL` secret only |
| `github-deploy` | the deploy workflow, on `main` only | `artifactregistry.writer` on the app's repository; `run.developer` on the app's service; `iam.serviceAccountUser` on `smoke-run` |
| `terraform-plan` | the PR plan check | `roles/viewer` and `roles/iam.securityReviewer` on the project; `storage.objectViewer` on the state bucket. Cannot change resources or read secret values |

The user runs `apply` as themselves (project owner) in Cloud Shell. No account with write access
to the infrastructure is usable from CI.

### 4.2 GitHub login (Workload Identity Federation)

- Pool `github`, OIDC provider `github-actions`, issuer
  `https://token.actions.githubusercontent.com`.
- Attribute mapping: `google.subject = assertion.sub`,
  `attribute.repository_id = assertion.repository_id`, `attribute.ref = assertion.ref`.
- Provider condition: `assertion.repository_id == '1416787003'`. The ID is used rather than the
  name because a name can be re-registered by someone else.
- `github-deploy` can be impersonated only with `attribute.ref = refs/heads/main`.
- `terraform-plan` can be impersonated by any workflow of the repository.
- If a second app later shares this Google project, the pool moves out of the module into
  `prod/`. Until then it stays in the module.

### 4.3 Artifact Registry

Repository `smoke-tracker`, Docker, `europe-west3`, cleanup policy *keep most recent versions:
3*, mode *delete*. Adopted from the hand-made repository with an `import` block.

### 4.4 Secret Manager

Secret `DATABASE_URL`, user-managed replication in `europe-west3`, **no version** in Terraform.
`lifecycle { prevent_destroy = true }`.

### 4.5 Cloud Run service

- Name `smoke-tracker`, region `europe-west3`, public (`allUsers` → `run.invoker`).
- Scaling 0 to 1 instances; CPU allocated only during requests; 256 MiB memory.
- Runs as `smoke-run`; `DATABASE_URL` from the secret's latest version.
- Initial image: Google's sample `us-docker.pkg.dev/cloudrun/container/hello`.
- `lifecycle { ignore_changes = [template[0].containers[0].image, client, client_version] }`:
  the deploy workflow changes the image, and Terraform must not roll it back.
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
- Before the first `apply` the GitHub variables don't exist; `plan` is skipped when
  `GCP_PROJECT_ID` is empty.
- Trivy findings that are intended (the service is public on purpose) are suppressed one by one,
  each with its reason, never with a blanket rule.
- Dependabot gains a `terraform` entry (weekly) for `infra/prod` and `infra/modules/cloud-run-app`.

Applying stays manual (§7).

---

## 6. GitHub repository variables

`GCP_PROJECT_ID`, `GCP_REGION`, `GCP_WIF_PROVIDER`, `GCP_DEPLOY_SA`, `GCP_PLAN_SA`,
`TF_STATE_BUCKET`. None is a secret. They are set by hand from the Terraform outputs.

---

## 7. Running it

**First time**

1. Create the state bucket: Cloud Storage, `europe-west3`, standard class, uniform access,
   public access prevention on, object versioning on.
2. In Cloud Shell: clone the repository, create `terraform.tfvars` and `backend.hcl` from the
   examples, `terraform init`, `terraform plan`, `terraform apply`.
3. Add the Neon connection string as the first version of the `DATABASE_URL` secret (console).
4. Set the GitHub variables from `terraform output`.
5. Open the service URL: Google's sample page appears.

**Routine:** PR → read the plan comment → merge → in Cloud Shell `git pull`, `terraform plan`
(should match the comment), `terraform apply`.

---

## 8. After milestone 3

Separate PRs, not part of this spec's implementation:

- `deploy.yml`: on push to `main`, build the image, push it to Artifact Registry and deploy it
  to Cloud Run as `github-deploy`.
- The app's settings as Cloud Run environment variables in `prod/`.
- Daily backup of the Neon database (Neon's free plan keeps 6 hours of history).

---

## 9. Verification

- CI: `check` passes; `plan` posts a comment on a test PR.
- After the first `apply`: a second `terraform plan` reports **no changes**; the service URL
  shows the sample page.
- The `import` adopts the existing Artifact Registry repository without replacing it (the plan
  shows *import*, not *create* or *replace*).

---

## 10. Documentation

- `infra/README.md`: §2 table, first-time setup, routine.
- `README.md`: Terraform in the hosting diagram; link to `infra/`.
- `docs/learning/hosting.md`: state, plan vs apply, imports, why some pieces stay manual.
- `docs/DECISIONS.md`: entries for Cloud Run + Neon (replacing #38 and #39), Terraform, state in
  GCS, Neon manual, apply from Cloud Shell. Numbered after milestone 3's #56–#73. Wording shown
  to the user before committing.
