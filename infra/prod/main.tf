locals {
  github_repository_id = "1416787003"
}

module "app" {
  source = "../modules/cloud-run-app"

  project_id           = var.project_id
  region               = var.region
  name                 = "smoke-tracker"
  github_repository    = "locadani/smoke-tracker"
  github_repository_id = local.github_repository_id

  # Cloud Run's default address is https://<service>-<project number>.<region>.run.app.
  env = {
    APP_BASE_URL     = "https://smoke-tracker-${data.google_project.this.number}.${var.region}.run.app"
    CLIENT_IP_HEADER = "X-Forwarded-For"
    RUST_LOG         = "info"
  }

  # Set to true in a PR once the secret has its first version (infra/README.md, first-time setup).
  attach_database_url = false

  depends_on = [google_project_service.apis]
}
