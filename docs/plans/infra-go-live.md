# Go-Live Infrastructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Everything needed to run Smoke Tracker in production, as code: the app's settings on Cloud Run, automatic deploys of one scanned image, a nightly database backup, a documented way to create the first user, and up-to-date docs.

**Architecture:** The `cloud-run-app` module gains plain environment settings and a backup (bucket, Cloud Run job running `pg_dump`, Cloud Scheduler trigger, its own account). `infra/prod` passes Smoke Tracker's settings. A new `deploy.yml` builds the image once, scans it, pushes it to Artifact Registry and updates the Cloud Run service; it replaces M3's GHCR push in `release.yml`. Docs catch up everywhere.

**Tech Stack:** Terraform 1.16.5, `hashicorp/google` ~> 8.6 (module `>= 8.0, < 9.0`), `terraform test` with `mock_provider`, GitHub Actions, gcloud (preinstalled on `ubuntu-latest`), Trivy (`aquasecurity/trivy-action@ed142fd0673e97e23eac54620cfb913e5ce36c25 # v0.36.0`), `postgres:18-alpine` for `pg_dump`.

**Spec:** `docs/specs/2026-10-13-hosting-infra-design.md` (§8 is replaced by this work; Task 4 updates the spec).

## Global Constraints

- Branch `infra/bootstrap-state` in worktree `/Users/daniele/Documents/Personal/smoke-tracker-hosting` (this PR also carries the bootstrap work already committed).
- Region `europe-west3`; app name `smoke-tracker`; repository `locadani/smoke-tracker`, ID `1416787003`.
- No secret value anywhere (repo, state, logs, PR comments, images). The database URL lives only in Secret Manager.
- Least privilege: grants on single resources where possible.
- Zero cost: free tiers; nothing with a minimum charge.
- Nothing installed on the dev machine: Terraform via `docker run --rm -v "$PWD:/repo" -w /repo/<dir> hashicorp/terraform:1.16.5 <args>`; actionlint via `rhysd/actionlint:1.7.12`; Trivy via `aquasec/trivy:0.75.0`. Never plan/apply against Google. Delete any `.terraform/` you create when done.
- Commits as the repo-local identity, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Never push.
- No tutorial comments; short "why" comments only.
- `deploy.yml` triggers only on `push: branches: [main]` (never `workflow_call`, `pull_request_target`, `workflow_dispatch`): the deploy account trusts `deploy.yml` on `main`.
- Never touch `claude.md`, `description.md`. The original spec `docs/specs/2026-10-13-smoke-tracker-design.md` may only receive short "superseded" notes (user-approved), no rewrites.

## Review Focus

1. Deploying the real image before `DATABASE_URL` is attached would make the app use a temporary SQLite file and lose data — `deploy.yml` must refuse (Task 3).
2. The backup job must not exist before the secret has a value (Cloud Run would fail to run it) — gated by `attach_database_url` (Task 2 test).
3. `APP_BASE_URL` must equal the service's real URL, or sign-in's Origin check fails — built from the project number (Task 1 test).
4. Deploy and plan workflows on fork, Dependabot or pre-setup pushes must skip, not fail (Task 3 gate).
5. Neon's connection string includes `channel_binding=require`; both the app (sqlx) and `pg_dump` must accept it (Task 1 check, Task 2 note).

---

### Task 1: App settings on the Cloud Run service

**Files:** Modify `infra/modules/cloud-run-app/variables.tf`, `service.tf`, `tests/cloud_run_app.tftest.hcl`; `infra/prod/main.tf`.

- [ ] **Step 1: Tests first.** Append:

```hcl
run "plain_settings_are_passed_to_the_container" {
  variables {
    env = {
      APP_BASE_URL     = "https://smoke-tracker-123.europe-west3.run.app"
      CLIENT_IP_HEADER = "X-Forwarded-For"
    }
  }

  assert {
    condition     = anytrue([for e in google_cloud_run_v2_service.app.template[0].containers[0].env : e.name == "CLIENT_IP_HEADER" && e.value == "X-Forwarded-For"])
    error_message = "Plain settings reach the container."
  }
  assert {
    condition     = length(google_cloud_run_v2_service.app.template[0].containers[0].env) == 2
    error_message = "Without attach_database_url, only the plain settings are set."
  }
}

run "settings_and_database_url_together" {
  variables {
    attach_database_url = true
    env                 = { RUST_LOG = "info" }
  }

  assert {
    condition     = length(google_cloud_run_v2_service.app.template[0].containers[0].env) == 2
    error_message = "One plain setting plus DATABASE_URL."
  }
}
```
Run module tests → RED (`env` undeclared).

- [ ] **Step 2: Code.** `variables.tf`:

```hcl
variable "env" {
  description = "Plain (non-secret) environment settings for the app. Secrets never go here."
  type        = map(string)
  default     = {}
}
```

In `service.tf`, inside `containers`, before the `DATABASE_URL` dynamic block:

```hcl
      dynamic "env" {
        for_each = var.env
        content {
          name  = env.key
          value = env.value
        }
      }
```

GREEN; existing tests still pass (the earlier `length(env) == 0` test keeps `env` at its default).

- [ ] **Step 3: Prod settings.** In `infra/prod/main.tf` module call add (after `github_repository_id`):

```hcl
  # Cloud Run's default address is https://<service>-<project number>.<region>.run.app.
  env = {
    APP_BASE_URL     = "https://smoke-tracker-${data.google_project.this.number}.${var.region}.run.app"
    CLIENT_IP_HEADER = "X-Forwarded-For"
    RUST_LOG         = "info"
  }
```
`APP_TIMEZONE` and `AUTO_CLOSE_HOUR` keep the app's defaults. `validate` prod.

- [ ] **Step 4: Check Neon's URL works with sqlx.** Neon's string looks like `postgresql://user:pass@ep-x.eu-central-1.aws.neon.tech/neondb?sslmode=require&channel_binding=require`. In the sqlx 0.9 source (`~/.cargo/registry` or the docs.rs page for `PgConnectOptions`), check whether an unknown/unsupported `channel_binding` query parameter is ignored or rejected. Report the finding with the source line. If rejected, Task 4 documents removing `&channel_binding=require` before storing the secret.

- [ ] **Step 5: fmt, tests, commit** `infra: app settings on Cloud Run (base URL, client IP header, log level)`.

---

### Task 2: Nightly database backup

**Files:** Create `infra/modules/cloud-run-app/backup.tf`; modify module `variables.tf`, `outputs.tf`, tests; `infra/prod/project.tf` (add `cloudscheduler.googleapis.com`).

Design: a private bucket `<project_id>-<name>-backups` (europe-west3, uniform access, public access prevention, objects deleted after `backup_retention_days` = 30); an account `<name>-backup` that can read the database secret, read/write that bucket, and run the backup job; a Cloud Run job (created only when `attach_database_url` is true) running `postgres:18-alpine` with the bucket mounted at `/backups`, writing `pg_dump --format=custom` files named with the UTC time; a Cloud Scheduler job (same condition) calling the job's `:run` endpoint daily at 03:00 Europe/Berlin with an OAuth token of the backup account.

- [ ] **Step 1: Tests first** (use `command = plan` where a resource has deletion protection, so test cleanup isn't blocked):

```hcl
run "backup_bucket_is_private_and_expires_old_dumps" {
  command = plan
  assert {
    condition     = google_storage_bucket.backups.name == "test-project-smoke-tracker-backups" && google_storage_bucket.backups.public_access_prevention == "enforced"
    error_message = "Private backups bucket named after project and app."
  }
  assert {
    condition     = anytrue([for r in google_storage_bucket.backups.lifecycle_rule : tolist(r.action)[0].type == "Delete" && tolist(r.condition)[0].age == 30])
    error_message = "Dumps older than 30 days are deleted."
  }
}

run "no_backup_job_before_the_secret_has_a_value" {
  command = plan
  assert {
    condition     = length(google_cloud_run_v2_job.backup) == 0 && length(google_cloud_scheduler_job.backup) == 0
    error_message = "The job needs DATABASE_URL, which exists only once attach_database_url is true."
  }
}

run "nightly_backup_when_attached" {
  command = plan
  variables {
    attach_database_url = true
  }
  assert {
    condition     = google_cloud_scheduler_job.backup[0].schedule == "0 3 * * *" && google_cloud_scheduler_job.backup[0].time_zone == "Europe/Berlin"
    error_message = "Runs daily at 03:00 Berlin time."
  }
  assert {
    condition     = google_cloud_run_v2_job.backup[0].template[0].template[0].service_account == google_service_account.backup.email
    error_message = "The job runs as the backup account."
  }
}

run "backup_account_grants_are_narrow" {
  command = plan
  assert {
    condition     = google_secret_manager_secret_iam_member.backup_reads_database_url.secret_id == google_secret_manager_secret.database_url.secret_id && google_storage_bucket_iam_member.backup_writes_dumps.bucket == google_storage_bucket.backups.name
    error_message = "The backup account reads one secret and writes one bucket."
  }
}
```
Add a file-level `override_resource` for `google_service_account.backup` (distinct, format-valid name/email/member, like the existing ones). RED.

- [ ] **Step 2: Code.** Variables: `backup_retention_days` (number, 30), `backup_image` (string, `"postgres:18-alpine"`; "pg_dump must be at least the server's major version; newer works"), `backup_schedule` (`"0 3 * * *"`), `backup_time_zone` (`"Europe/Berlin"`). Write `backup.tf` with: `google_storage_bucket.backups` (`force_destroy = false`, uniform access, `public_access_prevention = "enforced"`, `lifecycle_rule` Delete at `age = var.backup_retention_days`); `google_service_account.backup` (`account_id = "${var.name}-backup"`); `google_secret_manager_secret_iam_member.backup_reads_database_url` (secretAccessor); `google_storage_bucket_iam_member.backup_writes_dumps` (`roles/storage.objectUser` — Cloud Storage FUSE needs list/get as well as create); `google_cloud_run_v2_job.backup` (`count = var.attach_database_url ? 1 : 0`, `deletion_protection = false` so a mistake is fixable by re-apply, `template.template`: `service_account`, `execution_environment = "EXECUTION_ENVIRONMENT_GEN2"`, `max_retries = 1`, `timeout = "600s"`, container `image = var.backup_image`, `command = ["/bin/sh", "-c"]`, `args = ["pg_dump \"$DATABASE_URL\" --format=custom --no-owner --file=/backups/${var.name}-$(date -u +%Y%m%dT%H%M%SZ).dump"]` (escape `$` for Terraform as `$$` where needed), `env DATABASE_URL` from the secret's `latest`, `volume_mounts` `/backups`; `volumes` `gcs { bucket = google_storage_bucket.backups.name, read_only = false }`; `depends_on` both grants); `google_cloud_run_v2_job_iam_member.backup_runs_job` (`roles/run.invoker` for the backup account, count-gated); `google_cloud_scheduler_job.backup` (count-gated, `region = var.region`, schedule/time_zone vars, `attempt_deadline = "320s"`, `http_target` POST `https://run.googleapis.com/v2/projects/${var.project_id}/locations/${var.region}/jobs/${google_cloud_run_v2_job.backup[0].name}:run`, `oauth_token { service_account_email = google_service_account.backup.email }`). Check every field against the provider 8.6 docs (`cloud_run_v2_job`, `cloud_scheduler_job`, `storage_bucket`) before writing; confirm Cloud Scheduler is available in `europe-west3` (Google's Cloud Scheduler locations page) and report it. Outputs: `backups_bucket`, `backup_job_name` (`try(google_cloud_run_v2_job.backup[0].name, null)`). Prod: add `cloudscheduler.googleapis.com` to `local.apis`; add prod output `backups_bucket`.

- [ ] **Step 3: Redaction check.** The backups bucket name contains the project ID, so the existing redaction (longest-first) already covers it in plan comments — confirm with one extra case in `infra/scripts/redact-plan.test.sh` (`my-proj-123-smoke-tracker-backups` → fully `***`-ed).

- [ ] **Step 4: Verify** module tests, prod validate, fmt, Trivy config (suppress only intended findings with a reason), redaction test; delete `.terraform/`. Commit `infra: nightly pg_dump of the database to a private bucket, kept 30 days`.

---

### Task 3: `deploy.yml` — one build, scanned, deployed

**Files:** Create `.github/workflows/deploy.yml`; delete `.github/workflows/release.yml`; modify `.github/workflows/security.yml` (Trivy image job runs on `pull_request` and `schedule` only, since `deploy.yml` scans on `main`).

- [ ] **Step 1: Write `deploy.yml`:**

```yaml
name: Deploy

# Only a push to main: the deploy account trusts deploy.yml on main, and other triggers can
# run this file as if it came from main (infra/README.md, "Rules for the deploy workflow").
on:
  push:
    branches: [main]
    paths-ignore:
      - "docs/**"
      - "**/*.md"
      - "infra/**"

permissions:
  contents: read

concurrency:
  group: deploy-production
  cancel-in-progress: false

jobs:
  gate:
    name: Deploy prerequisites
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
            echo "GCP secrets are not set; skipping the deploy."
            echo "ready=false" >> "$GITHUB_OUTPUT"
          fi

  deploy:
    name: Build, scan, deploy
    needs: gate
    if: needs.gate.outputs.ready == 'true'
    runs-on: ubuntu-latest
    permissions:
      contents: read
      id-token: write
    env:
      PROJECT_ID: ${{ secrets.GCP_PROJECT_ID }}
      REGION: ${{ vars.GCP_REGION }}
      SERVICE: smoke-tracker
    steps:
      - uses: actions/checkout@v7
        with:
          persist-credentials: false

      - uses: google-github-actions/auth@v3
        with:
          workload_identity_provider: ${{ secrets.GCP_WIF_PROVIDER }}
          service_account: ${{ secrets.GCP_DEPLOY_SA }}

      # Without DATABASE_URL the image falls back to a SQLite file on Cloud Run's temporary disk.
      - name: Refuse to deploy before the database is connected
        run: |
          gcloud run services describe "$SERVICE" --region "$REGION" --format=json \
            | jq -e '.spec.template.spec.containers[0].env[]? | select(.name == "DATABASE_URL")' > /dev/null \
            || { echo "DATABASE_URL is not attached yet (infra/README.md, first-time setup)."; exit 1; }

      - uses: docker/setup-buildx-action@v4

      - name: Build
        uses: docker/build-push-action@v7
        with:
          context: .
          load: true
          tags: ${{ env.REGION }}-docker.pkg.dev/${{ env.PROJECT_ID }}/smoke-tracker/smoke-tracker:${{ github.sha }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

      - name: Scan the image and the crates compiled into it
        # Pinned to a commit, not a tag: a third-party action's tags can be moved.
        uses: aquasecurity/trivy-action@ed142fd0673e97e23eac54620cfb913e5ce36c25 # v0.36.0
        with:
          image-ref: ${{ env.REGION }}-docker.pkg.dev/${{ env.PROJECT_ID }}/smoke-tracker/smoke-tracker:${{ github.sha }}
          severity: CRITICAL,HIGH
          ignore-unfixed: true
          exit-code: "1"

      - name: Push
        run: |
          gcloud auth configure-docker "$REGION-docker.pkg.dev" --quiet
          docker push "$REGION-docker.pkg.dev/$PROJECT_ID/smoke-tracker/smoke-tracker:$GITHUB_SHA"

      # Only the image changes; every other setting is owned by Terraform.
      - name: Deploy
        run: |
          gcloud run services update "$SERVICE" --region "$REGION" \
            --image "$REGION-docker.pkg.dev/$PROJECT_ID/smoke-tracker/smoke-tracker:$GITHUB_SHA"
```

If `ubuntu-latest` lacks `gcloud`, add `google-github-actions/setup-gcloud@v3` after auth and report it. Check that `gcloud run services update --image` sends no labels that Terraform would see as drift (if it does, add them to `ignore_changes` in `service.tf` and report).

- [ ] **Step 2:** `git rm .github/workflows/release.yml`. In `security.yml`, add `if: github.event_name != 'push'` to the `trivy` job with a one-line comment (`# On main, deploy.yml scans the image it deploys.`).

- [ ] **Step 3: Verify** actionlint clean; Trivy config clean. Commit `ci: deploy.yml builds one image, scans it, pushes to Artifact Registry and updates Cloud Run` (body: replaces the GHCR push; refuses to deploy before DATABASE_URL is attached).

---

### Task 4: Docs

**Files:** `infra/README.md`, `README.md`, `docs/specs/2026-10-13-hosting-infra-design.md`, `docs/learning/hosting.md`, `docs/plans/ROADMAP.md`, `docs/plans/infra-terraform.md`, `docs/specs/2026-10-13-smoke-tracker-design.md` (notes only).

- [ ] `infra/README.md`:
  - Table: add rows "App settings (base URL, client IP header, log level) | Terraform (`prod/main.tf`)", "Nightly backup | Terraform (module `backup.tf`)", "Deploying the app | `deploy.yml` on every merge to `main`", "First resident | Manual, once, from Cloud Shell | The command asks for the password interactively".
  - After the attach step: **First deploy** — in GitHub → Actions → Deploy, open the most recent run (merging this PR created one, skipped while the secrets were missing) and click **Re-run all jobs**; a re-run keeps its original push trigger. Later merges deploy on their own. The deploy refuses until `DATABASE_URL` is attached.
  - **First resident** step (Cloud Shell): `gcloud auth configure-docker europe-west3-docker.pkg.dev`, then `docker run -it --rm -e DATABASE_URL="$(gcloud secrets versions access latest --secret=smoke-tracker-database-url)" europe-west3-docker.pkg.dev/YOUR_PROJECT_ID/smoke-tracker/smoke-tracker:LATEST_TAG create-resident YOUR_USERNAME` (tag = the commit SHA shown in the Deploy run or `gcloud artifacts docker images list`). Say the secret is passed only to that one process.
  - **Backups** section: where they are, 30-day retention, how to run one now (`gcloud run jobs execute smoke-tracker-backup --region europe-west3`), how to restore (`gcloud storage cp gs://…/FILE.dump .` then `pg_restore --clean --no-owner -d "$DATABASE_URL" FILE.dump` from Cloud Shell, which has the Postgres client — or via `docker run` of `postgres:18-alpine`), and that a restore overwrites live data.
  - If Task 1 found `channel_binding` rejected: in the secret step, say to remove `&channel_binding=require`.
  - "Changing the state bucket" → "Changing the bootstrap (bucket, budget)": PR first; CI validates and tests it but posts no plan; use `terraform plan -out=bootstrap.tfplan` / `terraform apply bootstrap.tfplan`. Step "Connect the secret" also uses `plan -out`/`apply <file>`.
  - Remove "not built yet" wording around `deploy.yml`; GitHub step: say `GCP_DEPLOY_SA`/`GCP_REGION` are used by `deploy.yml`.
  - One line: the gitignored `terraform.tfvars`/`backend.hcl` live only in Cloud Shell's home (deleted after ~120 days unused) — re-create them from the `.example` files.
- [ ] `README.md`: Container section — images go to Artifact Registry via `deploy.yml` (no GHCR); hosting table/diagram accurate (deploys now real; backup row/node added: Cloud Scheduler → backup job → backups bucket).
- [ ] Hosting spec: §6 "GitHub repository variables" heading/text and §5 "backend config from variables" → secrets; §3 lock files + gitignored files include `bootstrap/`; layout tree adds `scripts/` and `backup.tf`; "#56–#73" → "#56–#75"; §8 rewritten from "after milestone 3" to what is now built (settings, deploy.yml with the guard and single build, backup, first resident via Cloud Shell), with the `create-resident`-as-job idea removed and why.
- [ ] `docs/learning/hosting.md`: short "Deploying" and "Backups" bullets (one build = the scanned image is the deployed image; the guard; scheduler → job → bucket; restore overwrites).
- [ ] `docs/plans/ROADMAP.md`: M3 marked done; replace "Still open: choosing the hosting provider…" with "Hosting: Cloud Run + Neon, see the hosting spec and `infra/`"; M7 `migrate-to-postgres` marked "Not needed: production starts on Postgres (decision #77)" (keep the row, strike or annotate).
- [ ] `docs/plans/infra-terraform.md`: one line under the title: "> Superseded in part: the state bucket and budget are created by `infra/bootstrap/` (decision #82); app settings, deploys and backups by `docs/plans/infra-go-live.md`."
- [ ] Original spec (notes only, user-approved): at §10 SQLite-primary, §10.2/M7 migrate-to-postgres, §12 "deploy credentials in GitHub Secrets", §13.1 `release.yml`/GHCR, §13.2 hosting deferred + Litestream→R2, §1/§14 "hosting choice" out of scope — add a short italic line `> *Superseded: see docs/specs/2026-10-13-hosting-infra-design.md and decisions #76–#85.*` (adjust the pointer per place). Change nothing else.
- [ ] Verify: `grep -rn -E "GHCR|ghcr.io" README.md infra docs/specs/2026-10-13-hosting-infra-design.md` shows only historical/superseded mentions; no "Still open" in ROADMAP. Commit `docs: go-live docs — deploys, backups, first resident, roadmap and superseded notes`.

---

### Task 5: Decision log (user-approved wording)

Append after #82 in `docs/DECISIONS.md` the rows #83–#85 exactly as given in the dispatch (the controller supplies the approved text). Commit `docs(decisions): deploys, backups, first resident`.
