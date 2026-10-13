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
- `scripts/redact-plan.sh` — replaces the project ID, project number and state bucket name before a plan is posted on a PR.

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

## First-time setup

Done once, in the Google Cloud console and Cloud Shell.

1. **State bucket.** Cloud Storage → Create: region `europe-west3`, Standard class, uniform access,
   public access prevention on, object versioning on.
2. **Terraform in Cloud Shell** (it may be missing or an older version; the pinned one goes in `~/bin`, which survives between sessions):

   ```bash
   mkdir -p ~/bin && cd /tmp
   curl -fsSLO https://releases.hashicorp.com/terraform/1.16.5/terraform_1.16.5_linux_amd64.zip
   unzip -o terraform_1.16.5_linux_amd64.zip terraform -d ~/bin
   echo 'export PATH="$HOME/bin:$PATH"' >> ~/.bashrc && export PATH="$HOME/bin:$PATH"
   terraform version
   ```

3. **APIs Terraform needs to start:**

   ```bash
   gcloud config set project YOUR_PROJECT_ID
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

   If `apply` fails on a new service account's permissions (Google can take a minute to make a new
   account visible), run `terraform apply` again.

5. **Database secret.** Secret Manager → `smoke-tracker-database-url` → New version → paste the Neon
   *direct* connection string. Don't save it anywhere else.
6. **GitHub** (Settings → Secrets and variables → Actions), from `terraform output` unless noted:
   - Secrets: `GCP_PROJECT_ID` (the value in `terraform.tfvars`), `GCP_PROJECT_NUMBER` (`project_number`), `GCP_WIF_PROVIDER`
     (`workload_identity_provider`), `GCP_DEPLOY_SA` (`deploy_service_account`), `GCP_PLAN_SA`
     (`plan_service_account`), `TF_STATE_BUCKET` (the value in `terraform.tfvars`).
   - Variable: `GCP_REGION` = `europe-west3`.

   These are identifiers, not credentials; they are secrets so GitHub masks them in public logs.
7. **Connect the secret.** A PR sets `attach_database_url = true` in `prod/main.tf`; merge, then in Cloud Shell
   `cd ~/smoke-tracker && git pull && cd infra/prod && terraform apply`. The plan shows `DATABASE_URL`
   being added to the service.
8. **Check.** Open `terraform output service_url`: Google's sample page. A second
   `terraform plan` reports no changes.

The first plan comment on a PR may fail with "permission denied" for a few minutes after the
first apply, while permissions spread. Re-run the job.

## Rules for the deploy workflow

`deploy.yml` must run only on `push: branches: [main]`. Never add `workflow_call`,
`pull_request_target` or `workflow_dispatch`. The deploy account trusts "`deploy.yml` on `main`",
and those triggers can also run that file as if it came from `main`.

## Changing the database password

Add a new version of `smoke-tracker-database-url`, **destroy** the old one (disabled versions still
count towards the 6 free ones), then redeploy: instances read the secret when they start.
