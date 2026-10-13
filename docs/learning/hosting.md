# Hosting: what I learned setting it up

How the app is hosted is in the [README](../../README.md#hosting). This note keeps the *why*
behind each choice, and where to find the answers without help.

## Choosing a host

| Option | Good | Bad |
|---|---|---|
| **Own VM** (Oracle Always Free) | Always on, so always instant; one provider; teaches Linux, networking, HTTPS | You patch, back up and rebuild it yourself; free VMs can be reclaimed when idle |
| **Container platform + managed database** (Cloud Run + Neon) | Nothing to maintain; HTTPS included. Neon Free keeps only 6 hours of restore history, so a daily backup is still needed | Two providers; the app sleeps when idle, so the first request after a pause waits a moment (a *cold start*) |
| Render / Koyeb free tiers | Simple | Render takes ~1 minute to wake; Koyeb allows one free app |

**Cold start:** a platform that charges only for requests stops the app when nobody uses it, and
starts it again on the next request. A small Rust binary starts in about a second; many
languages take several. A VM has no cold start because it never stops, which is also why it
needs looking after.

**Chosen:** Cloud Run + Neon, to get live quickly. If cold starts feel slow, move to the VM. The
same Dockerfile works on both, but Cloud Run needs an `amd64` image and Oracle's free VMs are
ARM (`arm64`), so moving means building for both.

## Secrets stay outside the app

- The app is the same everywhere. Settings, including secrets, are handed to it **from outside
  when it starts** (environment variables). Locally `DATABASE_URL` points to a local database; on
  Google it points to Neon.
- **Never build a secret into the image.** Anyone with the image can read it, every old image
  keeps a copy, and changing the password would mean rebuilding.
- **Secret Manager instead of a GitHub secret:** a GitHub secret would be sent to Cloud Run on
  every deploy and then shown as plain text in the service settings. In Secret Manager the
  password lives in one encrypted place, Cloud Run reads it at startup, and GitHub never sees it.
- **No stored key for deploys:** Workload Identity Federation lets GitHub prove which repository
  is running the workflow and get a short-lived Google token. There is no long-lived key to leak.
  The condition uses the repository *ID*, not its name, because a name can be re-registered by
  someone else.

## Regions

- **Put the app next to its database; make everything else match.** The browser talks to the
  app once per page, but the app talks to the database many times per page. With the app in
  Berlin (`europe-west10`) and Neon in Frankfurt, every query would pay the trip again.
- Keep the image registry in the same region as Cloud Run: faster deploys, and no transfer cost.
- Some choices can't be changed later (Neon's project region, Oracle's home region), so pick
  them before creating anything.

## Google Cloud basics

- A new project has every service (API) switched off. Enable only what you use. In the API
  Library, pick the result whose name ends in **API**; the other result is the product overview.
- Set a **budget alert** (€1) as soon as billing is linked.
- Give the image registry a **cleanup policy**. Every deploy adds a ~20 MB image and nothing is
  deleted by default, so the free 0.5 GB fills up in a few weeks. It takes **two rules**: a
  *delete* rule (delete old images) and a *keep* rule (keep the 3 most recent). A keep rule on its
  own deletes nothing; it only protects images from delete rules. Keeping 3 still allows a
  one-click rollback. Policies run about once a day, in mode "delete", not "dry run".
- Error messages usually say what is missing, with a link to fix it, for example
  "Cloud Run Admin API has not been used in project … enable it by visiting …".

## Terraform

- **Plan vs apply.** `plan` compares the code with reality and lists what would change; `apply`
  makes the change. Here CI only plans (read-only account); applying is done by hand.
- **State.** After each apply Terraform records what it manages in a state file, kept in a
  versioned bucket. Anything Terraform manages ends up in it, which is why secret values stay
  out of Terraform.
- **Import.** Something created by hand can be adopted with an `import` block instead of being
  recreated. The plan says "will be imported".
- **`ignore_changes`.** When something else owns an attribute (the deploy workflow owns the
  image), Terraform must ignore it, or every apply would undo the deploy.
- **Modules.** `modules/cloud-run-app` describes one app; `prod/` uses it. A second app reuses the
  module with different inputs.
- **Tests without a cloud account.** `terraform test` with `mock_provider` checks the
  configuration (names, permissions, settings) offline, in CI.
- **Least privilege.** Each grant is on one resource where possible (one secret, one registry,
  one service), not on the whole project.

## Where to find this without help

- Cloud Run quickstart: <https://cloud.google.com/run/docs/quickstarts>
- Keyless login from GitHub: the README of
  [`google-github-actions/auth`](https://github.com/google-github-actions/auth)
- Secrets in Cloud Run: <https://cloud.google.com/run/docs/configuring/services/secrets>
- Artifact Registry repositories and cleanup policies:
  <https://cloud.google.com/artifact-registry/docs/repositories/create-repos>,
  <https://cloud.google.com/artifact-registry/docs/repositories/cleanup-policy>
- Neon plans and connecting: <https://neon.com/docs/introduction/plans>,
  <https://neon.com/docs/connect/connect-intro>
- Cloud Run regions and pricing tiers: <https://cloud.google.com/run/docs/locations>
- Terraform tests and mocks: <https://developer.hashicorp.com/terraform/language/tests/mocking>
- Import blocks: <https://developer.hashicorp.com/terraform/language/import>
- Google provider docs: <https://registry.terraform.io/providers/hashicorp/google/latest/docs>

## Cost

"Free" here means **about €0.00–0.01 a month**, not a hard zero:

- Cloud Run, Secret Manager (up to 6 active secret versions), Artifact Registry (0.5 GB) and Neon
  are inside their free tiers.
- Google's free outgoing traffic covers North America only, so pages sent from Frankfurt, and
  queries to Neon (on AWS), cost a fraction of a cent.
- Storage buckets outside the US are not in the free tier; a few KB costs nothing in practice.
- A **budget alert only warns**, it never stops spending. At most one running instance caps the
  compute; traffic is not capped.
- When the database password changes, destroy the old secret version: disabled versions still
  count towards the 6 free ones.

## Corrected during setup

A review against the official docs caught these; worth remembering when following old tutorials:

- Terraform is **not** preinstalled in Cloud Shell any more (since June 2026). Check release
  notes rather than old tutorials.
- A *keep 3* cleanup rule alone deletes nothing (see above).
- Neon's pooled connection does handle prepared statements (PgBouncer 1.22+). The direct
  connection is still used, because the app has its own pool and migrations need a direct
  connection.
