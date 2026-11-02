# Runbook: PM2.5 photo

Adds the private photo bucket (Frankfurt) and Cloud Vision reading. All Google configuration is Terraform; nothing is set by hand in a console.

1. **Cloud Vision API.** Enabled by Terraform (`infra/prod/project.tf`). The app calls the EU endpoint `eu-vision.googleapis.com` with its own service account, so no key is created.
2. **Budget alert.** Managed by Terraform (`infra/bootstrap/budget.tf`); nothing to do. Vision's first 1,000 reads per month are free.
3. **Deploy.** Apply the infra change as `infra/README.md` describes (PR, plan, apply on merge). It enables Vision, creates `smoke-tracker-photos-<project>`, grants the runtime account create and read only, and sets `PHOTO_BUCKET` and `PM25_READER=vision`. Then deploy the image: GitHub, Actions, Deploy, **Re-run all jobs**. Agents never merge.
4. **Bucket check (optional, read-only).** Cloud Storage, the photos bucket: location `europe-west3`, public access prevention on, one lifecycle rule (delete `pending/` after 1 day).
5. **Android.** Chrome: log a PM2.5 reading. The app's own camera opens, the torch toggle shows, a sideways photo still reads. Confirm the value, save, open the entry, the photo shows.
6. **iPhone.** Safari: the camera app path opens (no in-page camera). Safari must not offer the torch. Take the photo, confirm the value, save.
7. **Audit.** Open Cloud Shell and run, as in `docs/runbooks/0005-neon.md` step 5, `verify-audit`. It must report the chain clean.

**Rollback.** Terraform only: set `PM25_READER = "none"` in `infra/prod/main.tf`, open a PR, apply on merge. Readings go back to manual entry. Stored photos stay. Do not edit the Cloud Run service in the console.

**Orphan photo file.** `verify-audit` reporting `photo file without entry: <key>` means a file is in the bucket that no entry points to (an upload whose save failed). Check the key is not referenced by any entry, then delete the file in the Cloud Storage console. This is a one-off data cleanup, not configuration; the app's service account cannot delete.
