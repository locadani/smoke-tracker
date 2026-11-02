# Runbook: PM2.5 photo

Adds the private photo bucket (Frankfurt) and Cloud Vision reading. Everything below uses web consoles only.

1. **Enable Cloud Vision.** Google Cloud console, APIs and Services, Library, search **Cloud Vision API**, Enable. The app calls the EU endpoint `eu-vision.googleapis.com` with its own service account, so no key is created.
2. **Budget alert.** Billing, Budgets and alerts: confirm the existing alert is still there. Vision's first 1,000 reads per month are free.
3. **Deploy.** Apply the infra change as `infra/README.md` describes (PR, plan, apply on merge). It creates `smoke-tracker-photos-<project>`, grants the runtime account create and read only, and sets `PHOTO_BUCKET` and `PM25_READER=vision`. Then deploy the image: GitHub, Actions, Deploy, **Re-run all jobs**. Agents never merge.
4. **Bucket check.** Cloud Storage, the photos bucket: location `europe-west3`, public access prevention on, one lifecycle rule (delete `pending/` after 1 day).
5. **Android.** Chrome: log a PM2.5 reading. The app's own camera opens, the torch toggle shows, a sideways photo still reads. Confirm the value, save, open the entry, the photo shows.
6. **iPhone.** Safari: the camera app path opens (no in-page camera). Safari must not offer the torch. Take the photo, confirm the value, save.
7. **Audit.** Open Cloud Shell and run, as in `docs/runbooks/0005-neon.md` step 5, `verify-audit`. It must report the chain clean.

**Rollback.** Set `PM25_READER` to `none` in the Cloud Run service (Edit and deploy new revision); readings go back to manual entry. Stored photos stay.
