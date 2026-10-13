locals {
  github_repository_id = "1416787003"

  # Cloud Run's default address is https://<service>-<project number>.<region>.run.app.
  app_base_url = "https://smoke-tracker-${data.google_project.this.number}.${var.region}.run.app"
}

module "app" {
  source = "../modules/cloud-run-app"

  project_id           = var.project_id
  region               = var.region
  name                 = "smoke-tracker"
  github_repository    = "locadani/smoke-tracker"
  github_repository_id = local.github_repository_id

  env = {
    APP_BASE_URL     = local.app_base_url
    CLIENT_IP_HEADER = "X-Forwarded-For"
    RUST_LOG         = "info"
  }

  # Set to true in a PR once the secret has its first version (infra/README.md, first-time setup).
  attach_database_url = true

  depends_on = [google_project_service.apis]
}
