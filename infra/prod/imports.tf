# Created by hand in the console before Terraform; adopted instead of recreated.
# Once imported, this block has no further effect.
import {
  to = module.app.google_artifact_registry_repository.images
  id = "projects/${var.project_id}/locations/${var.region}/repositories/smoke-tracker"
}
