# Infrastructure

Smoke Tracker's Google Cloud setup, as Terraform. Design:
[hosting infrastructure spec](../docs/specs/2026-10-13-hosting-infra-design.md).

## What is automatic and what is manual

| Piece | How | Why |
|---|---|---|
| Google project, billing account | Manual | Terraform needs a project with billing before it can run |
| €1 budget alert | Terraform (`bootstrap/`) | Applied by hand with the bootstrap, not by CI: billing permissions stay out of the pipeline |
| State bucket | Terraform (`bootstrap/`), first run with a local state | Terraform needs somewhere to keep its state, so this small configuration creates the bucket, then moves its own state into it |
| Neon project (Frankfurt) | Manual | No official Terraform provider; the community one can replace resources on upgrade, which for a database means losing data |
| Database connection string (the secret's value) | Manual, once | A value managed by Terraform is stored in its state in plain text |
| GitHub secrets and variable | Manual, once | Setting them from Terraform would need a GitHub admin token |
| Project APIs, image registry, secret (without value), service accounts and grants, GitHub login, Cloud Run service | Terraform | Reviewed in PRs, repeatable |
| App settings (base URL, client IP header, log level) | Terraform (`prod/main.tf`) | Reviewed in PRs, repeatable |
| Nightly backup | Terraform (module `backup.tf`) | Bucket, job, schedule and their grants are code like the rest |
| Deploying the app | `deploy.yml` on every push to `main` that changes the app (changes only to `docs/`, `*.md` or `infra/` don't deploy) | Terraform owns the settings; the workflow only changes which image runs |
| First resident | Manual, once, from Cloud Shell | The command asks for the password interactively |

## Layout

- `bootstrap/` — the state bucket and the €1 budget alert. Run once; its own state then lives in the bucket it created.
- `modules/cloud-run-app/` — one app on Cloud Run; reusable. Tests: `terraform test` (mocked provider).
- `prod/` — Smoke Tracker's production: APIs, the module, the read-only `terraform-plan` account.
- `scripts/redact-plan.sh` — replaces the project ID, project number and state bucket name before a plan is posted on a PR.

The `terraform.tfvars` and `backend.hcl` files are gitignored, so they live only in Cloud Shell's
home directory, which Google deletes after about 120 days without use. Re-create them from the
`.example` files.

Good to know: deleting the GitHub login pool keeps its name reserved for 30 days, so a destroy
followed by an apply fails until the pool is undeleted.

## Every change

1. Open a PR that changes `infra/`. CI checks, tests and scans it, then posts a plan comment.
2. Read the plan. Anything marked *destroy* or *replace* needs a reason.
3. Merge, then in Cloud Shell:

   ```bash
   cd ~/smoke-tracker && git pull
   cd infra/prod
   terraform plan -out=prod.tfplan    # should match the PR comment
   terraform apply prod.tfplan        # applies exactly what you reviewed
   ```

## Changing the bootstrap (bucket, budget)

Open a PR first. CI validates and tests `bootstrap/` but posts no plan (the read-only plan account
has no billing-budget permissions; it can read the bootstrap state, which holds only bucket and
budget settings). After merging, in Cloud Shell:

```bash
cd ~/smoke-tracker && git pull && cd infra/bootstrap && terraform init -backend-config=backend.hcl
terraform plan -out=bootstrap.tfplan
terraform apply bootstrap.tfplan
```

The bucket can't be deleted or replaced by Terraform (`prevent_destroy`), so a change that would
replace it (name, region) stops at plan; and every state file keeps the current state plus 9 older versions, which
can be restored from the bucket's "Noncurrent versions" in the console.

## First-time setup

Done once, in the Google Cloud console and Cloud Shell.

1. **Terraform in Cloud Shell** (it may be missing or an older version; the pinned one goes in `~/bin`, which survives between sessions):

   ```bash
   mkdir -p ~/bin && cd /tmp
   curl -fsSLO https://releases.hashicorp.com/terraform/1.16.5/terraform_1.16.5_linux_amd64.zip
   unzip -o terraform_1.16.5_linux_amd64.zip terraform -d ~/bin
   echo 'export PATH="$HOME/bin:$PATH"' >> ~/.bashrc && export PATH="$HOME/bin:$PATH"
   terraform version
   ```

2. **APIs Terraform needs to start:**

   ```bash
   gcloud config set project YOUR_PROJECT_ID
   gcloud services enable serviceusage.googleapis.com cloudresourcemanager.googleapis.com storage.googleapis.com
   ```

   The bootstrap enables the Billing Budgets API itself.

3. **Clone the repository:**

   ```bash
   git clone https://github.com/locadani/smoke-tracker.git ~/smoke-tracker
   ```

4. **State bucket (bootstrap).** Until the check in (iii) passes, the local state file is the only
   record of the bucket, so do (ii) and (iii) in one sitting.

   (i) Copy the two example files and **edit them** (project ID, billing account ID from Billing → Account
   management, and a bucket name such as `<project-id>-tfstate`; the bucket name is the same in both), then stop and check them. The billing account's currency must be EUR (otherwise change the currency in
   `budget.tf`), and you need Billing Account Administrator (or Costs Manager) on it; the account owner has it:

   ```bash
   cd ~/smoke-tracker/infra/bootstrap
   cp terraform.tfvars.example terraform.tfvars
   cp backend.hcl.example backend.hcl
   ```

   (ii) Create the bucket with a local state (type `yes` at the apply prompt):

   ```bash
   printf 'terraform {\n  backend "local" {}\n}\n' > backend_override.tf
   terraform init
   terraform apply
   ```

   Then move the state into the bucket (type `yes` when asked):

   ```bash
   rm backend_override.tf
   terraform init -migrate-state -backend-config=backend.hcl
   ```

   (iii) Check: `terraform plan` says "No changes", and
   `gcloud storage ls gs://YOUR_BUCKET/smoke-tracker/bootstrap/` lists `default.tfstate`.

   (iv) Only then, as a separate command: `rm -f terraform.tfstate terraform.tfstate.backup`.

   The bootstrap also creates the €1 budget alert. If you created a budget by hand earlier, delete it
   (Billing → Budgets & alerts) so alerts don't arrive twice.

   If the local state is lost before (iii), import both, otherwise the next apply creates a second budget:
   `terraform import google_storage_bucket.state YOUR_BUCKET` and
   `terraform import google_billing_budget.monthly billingAccounts/YOUR_BILLING_ACCOUNT_ID/budgets/BUDGET_ID`
   (the budget ID is in the budget's URL in Billing → Budgets & alerts).

5. **Configure and apply production:**

   ```bash
   cd ~/smoke-tracker/infra/prod
   cp terraform.tfvars.example terraform.tfvars   # fill in project ID and the bucket name from step 4
   cp backend.hcl.example backend.hcl             # fill in the bucket name from step 4
   terraform init -backend-config=backend.hcl
   terraform plan    # the registry shows "will be imported", not "created"
   terraform apply
   ```

   If `apply` fails on a new service account's permissions (Google can take a minute to make a new
   account visible), run `terraform apply` again.

6. **Database secret.** Secret Manager → `smoke-tracker-database-url` → New version → paste the Neon
   *direct* connection string. Don't save it anywhere else. It contains `sslmode=require`, so the
   connection is encrypted. Leave `channel_binding=require` in; the app ignores it with a warning.
7. **GitHub** (Settings → Secrets and variables → Actions), from `terraform output` unless noted:
   - Secrets: `GCP_PROJECT_ID` (the value in `terraform.tfvars`), `GCP_PROJECT_NUMBER` (`project_number`), `GCP_WIF_PROVIDER`
     (`workload_identity_provider`), `GCP_DEPLOY_SA` (`deploy_service_account`), `GCP_PLAN_SA`
     (`plan_service_account`), `TF_STATE_BUCKET` (`terraform output state_bucket` in `bootstrap/`).
   - Variable: `GCP_REGION` = `europe-west3`.

   `deploy.yml` uses `GCP_PROJECT_ID`, `GCP_WIF_PROVIDER`, `GCP_DEPLOY_SA` and the variable
   `GCP_REGION`. `infra.yml` uses `GCP_PROJECT_ID`, `GCP_PROJECT_NUMBER`, `GCP_WIF_PROVIDER`,
   `GCP_PLAN_SA` and `TF_STATE_BUCKET`.

   These are identifiers, not credentials; they are secrets so GitHub masks them in public logs.
8. **Connect the secret.** A PR sets `attach_database_url = true` in `prod/main.tf`; merge, then in Cloud Shell
   `cd ~/smoke-tracker && git pull && cd infra/prod`, then `terraform plan -out=attach.tfplan` and
   `terraform apply attach.tfplan`. The plan shows `DATABASE_URL` being added to the service, and the
   backup job and its schedule being created.
9. **First deploy.** In GitHub, open Actions → Deploy and the most recent run (merging the PR
   that added `deploy.yml` created one, skipped while the secrets were missing), then click
   **Re-run all jobs**. A re-run keeps its original push trigger. Later merges deploy on their
   own. The deploy refuses until `DATABASE_URL` is attached (step 8). GitHub allows re-runs only for
   30 days; if the run is older, merge any small change outside `docs/`, `*.md` and `infra/`.
10. **First resident.** In Cloud Shell, using the image the service runs:

    ```bash
    gcloud auth configure-docker europe-west3-docker.pkg.dev --quiet
    IMAGE="$(gcloud run services describe smoke-tracker --region europe-west3 --format='value(spec.template.spec.containers[0].image)')"
    DATABASE_URL="$(gcloud secrets versions access latest --secret=smoke-tracker-database-url)" \
    docker run -it --rm -e DATABASE_URL "$IMAGE" create-resident YOUR_USERNAME
    ```

    The command asks for the password. `-e DATABASE_URL` without a value takes it from the
    environment, so the connection string never appears on a command line; it reaches only that
    one process and is not saved.
11. **Check.** Open `terraform output service_url` (the address the app is configured with; its
    Origin check requires it), sign in as the resident, and run a second
    `terraform plan`: it reports no changes.

The first plan comment on a PR may fail with "permission denied" for a few minutes after the
first apply, while permissions spread. Re-run the job.

## Backups

A Cloud Run job, `smoke-tracker-backup`, runs every day at 03:00 (Europe/Berlin), started by Cloud
Scheduler. It runs `pg_dump` against the Neon database and writes `smoke-tracker-<UTC time>.dump`
to the bucket shown by `terraform output backups_bucket` (`<project>-smoke-tracker-backups`,
`europe-west3`). A dump is written as `.dump.partial` and renamed when it succeeds, so a
leftover `.partial` file is a failed run; ignore it. Dumps are deleted after 30 days. The job
exists only once `attach_database_url` is `true`. The bucket is in the EU, so it is outside the
free storage tier; a few dumps cost fractions of a cent.

Run one now:

```bash
gcloud run jobs execute smoke-tracker-backup --region europe-west3
```

Restore, from Cloud Shell, with Docker so the client matches the dump's version.
**This overwrites the live data.** Replace `FILE.dump` with a name from the listing.

```bash
gcloud storage ls gs://YOUR_BACKUPS_BUCKET/
gcloud storage cp gs://YOUR_BACKUPS_BUCKET/FILE.dump .
DATABASE_URL="$(gcloud secrets versions access latest --secret=smoke-tracker-database-url)" \
docker run --rm -e DATABASE_URL -v "$PWD":/restore postgres:18-alpine \
  sh -c 'pg_restore --clean --if-exists --no-owner --single-transaction -d "$DATABASE_URL" /restore/FILE.dump'
```

## Rules for the deploy workflow

`deploy.yml` must run only on `push: branches: [main]`. Never add `workflow_call`,
`pull_request_target` or `workflow_dispatch`. The deploy account trusts "`deploy.yml` on `main`",
and those triggers can also run that file as if it came from `main`.

## Changing the database password

Add a new version of `smoke-tracker-database-url`, **destroy** the old one (disabled versions still
count towards the 6 free ones), then redeploy: instances read the secret when they start.
