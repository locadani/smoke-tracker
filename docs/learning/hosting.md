# Hosting: what I learned setting it up

How the app is hosted is in the [README](../../README.md#hosting). This note keeps the *why*
behind each choice, and where to find the answers without help.

## Choosing a host

| Option | Good | Bad |
|---|---|---|
| **Own VM** (Oracle Always Free) | Always on, so always instant; one provider; teaches Linux, networking, HTTPS | You patch, back up and rebuild it yourself; free VMs can be reclaimed when idle |
| **Container platform + managed database** (Cloud Run + Neon) | Nothing to maintain; HTTPS and backups included | Two providers; the app sleeps when idle, so the first request after a pause waits a moment (a *cold start*) |
| Render / Koyeb free tiers | Simple | Render takes ~1 minute to wake; Koyeb allows one free app |

**Cold start:** a platform that charges only for requests stops the app when nobody uses it, and
starts it again on the next request. A small Rust binary starts in about a second; many
languages take several. A VM has no cold start because it never stops, which is also why it
needs looking after.

**Chosen:** Cloud Run + Neon, to get live quickly. If cold starts feel slow, move to the VM. The
same Docker image runs on both.

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
- Give the image registry a **cleanup policy** (keep the 3 most recent versions, mode "delete",
  not "dry run"). Every deploy adds a ~20 MB image and nothing is deleted by default, so the free
  0.5 GB fills up in a few weeks. Keeping 3 still allows a one-click rollback to an earlier
  version. The policy runs about once a day.
- Error messages usually say what is missing, with a link to fix it, for example
  "Cloud Run Admin API has not been used in project … enable it by visiting …".

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
