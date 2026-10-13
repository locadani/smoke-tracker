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
