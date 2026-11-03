# Deleting entries and events — design

Status: approved 2026-11-03. Built on branch `feat/delete-entries-events`; see "Decided during
build" at the end.

## Why

On 2026-11-03 Daniele logged the same night twice: entry 63 (07:15, no PM2.5) opened event 27,
and entry 64 (backdated to 01:17, PM2.5 20) opened event 28. There is no way in the app to
remove the mistaken entry and its event. Editing Neon by hand breaks `verify-audit`.

## Goals

- Delete one of your own log entries from its "Registrazione" page.
- Delete a whole event (and its entries) from its "Evento del …" page, when every live entry in
  it is yours.
- When someone else also logged in the event, say so briefly instead of deleting.
- Every deletion goes through the audited store, so `verify-audit` stays clean.

## Non-goals

- Undo / restore. Deleted stays deleted (the audit log keeps the history).
- Deleting other people's entries, or deleting an event for someone else.
- Removing PM2.5 photos from Cloud Storage. A deleted entry's photo stays, linked to the deleted
  entry; only the audit log reaches it.
- Hard deletes of any row.

## Anti-goals

- "Closed" and "deleted" must never look alike. Deleting never touches `closed_at` /
  `close_reason`; closing never deletes.
- No second copy of the truth (no stored "deleted" or "live entries" field on events).
- Deleting is never more permissive than the rule below (no deleting another resident's entry).

## The model

- **An event exists only while it has at least one live entry** (`log_entries.deleted_at IS
  NULL`). No column is added to `events`. An event whose entries are all deleted is simply not
  an event: History, graphs, the event banner, the auto-close notice, "Which event was this?"
  choices, the open-event check and `/events/{id}` all ignore it (`/events/{id}` answers "Page
  not found").
- **Deleting an entry** = the existing audited `delete_entry` (sets `log_entries.deleted_at`,
  one `log_entry` `delete` audit row). If it was the event's last live entry, the event is gone
  with it — nothing else is written. If it was the earliest of several, the event is
  recalculated (below).
- **Deleting an event** = deleting all its live entries in one transaction, one audit row each.
- A deleted entry's smell report and PM2.5 reading are hidden with it (already the case).
- An entry with no event ("No smell" check) is deleted the same way; nothing else changes.
- "Move to a new event" that takes the last entry out of an event follows the same rule: the old
  event stops existing.

## Only one open event

Today the database enforces it with the partial unique index `events_one_open` on
`closed_at IS NULL`. That index cannot see entries, so an open event whose entries were all
deleted would block every new event.

Decision (Daniele): **the app enforces the rule** and a migration drops `events_one_open` (both
Postgres and SQLite). Wherever an event is opened or reopened, the store checks — inside the
write transaction, which already holds the exclusive writer lock (`LOCK TABLE audit_log IN
EXCLUSIVE MODE`, `crates/storage-postgres/src/lib.rs`) — that no other open event with a live
entry exists, and keeps returning `Conflict::AnotherEventOpen` otherwise. "Is there an open
event?" everywhere means "an open event with a live entry". Contract tests cover: two opens
refused, reopen refused while another is open, an open event emptied by deletion no longer
blocks a new one. The migration is safe on live Neon data (index drop only). SQLite must hold an
equivalent writer lock for the check to be race-free; the builder confirms it does.

An emptied event that was open keeps `closed_at IS NULL` in the database. It is never closed by
the deletion and never shown.

## The event follows its remaining entries (Daniele: option b)

When a deletion (or a move) leaves an event with live entries but removes its earliest one, the
event is recalculated in the same transaction:

- `opened_at` becomes the earliest remaining live entry's `observed_at`;
- `owner_id` becomes that entry's author (so the event can change hands, e.g. become Duygu's);
- `closed_at` / `close_reason` are not touched (closing stays its own decision). Claude's call:
  if the new start would fall after `closed_at` (possible with backdated entries), `opened_at`
  stops at `closed_at`.

This is an audited `update` of the event (one `event` audit row), so `verify-audit` stays clean.
The event title, History order and graph ranges follow automatically.

## Who may delete (Daniele: option b)

- **Entry:** its author, or a resident when the author is a guest — never another resident's
  entry. In both cases only while the person deleting can log (`can(actor, LogEntry)`; viewers
  and expired guests cannot). A guest whose access expired leaves entries a resident can still
  remove.
- **Event:** only when the person deleting may delete every live entry in it (their own, plus
  guests' for a resident). Who opened the event does not matter.

## Screens

Italian shown; English and Turkish get the same keys (it/tr drafts need native review, as
usual). No browser dialogs: confirmation is an in-page step.

**Event page** — an `Elimina` button next to `Riapri` / `Chiudi`.

Only your entries, after tapping Elimina:

```
Eliminare questo evento e le sue 3 registrazioni?
Non si può annullare.
[ Elimina ]  [ Annulla ]
```

(One entry: "Eliminare questo evento e la sua registrazione?")

Someone else also logged — Elimina greyed out, recap below it:

```
4 registrazioni: tu 3 · Duygu 1
Per eliminare l'evento, Duygu deve prima eliminare le sue.
```

Three or more people:

```
5 registrazioni: tu 2 · Duygu 2 · Marco 1
Per eliminare l'evento, gli altri devono prima eliminare le loro.
```

Recap rules: counts live entries only; "tu" first, then others by count (most first); shown
only when the button is greyed out. The "deve prima eliminare" sentence names only the people
whose entries block you (for a resident, guests never block). If you may delete none of the
entries in the event, the button is not shown at all.

**Entry page ("Registrazione")** — an `Elimina` button next to `Correggi un errore`, on entries
you may delete (also on entries with no smell report, where `Correggi un errore` is absent).

```
Eliminare questa registrazione? Non si può annullare.
[ Elimina ]  [ Annulla ]
```

**After deleting:** an entry → its event page, or History (`/events`) if the event no longer
exists or the entry had none. An event → History.

## Builder-check answers (Claude's calls, for Daniele to check)

- **Every event query ignores emptied events**, in both backends: the lists, the "event at time
  T" and overlap-window lookups (`events.rs`), the open-event check, auto-close at 04:00, and
  the closing of the previous open event on "new event" / backdated entries. An emptied open
  event is therefore never auto-closed and never captures a new entry.
- **One store helper decides "the open event"** (open and has a live entry), and every path that
  opens or reopens an event (Log with a new event, backdated Log, "Move to a new event", Riapri)
  goes through it. Contract tests per path are the guard; no extra runtime assertion.
- **`verify-audit`** compares every record with its latest audit entry. A deletion writes one
  `log_entry` row per entry, plus one `event` `update` row when the event is recalculated; a
  whole-event delete writes all of them in one transaction or none.
- **Already deleted** (double tap, or someone else deleted first): the POST answers "Page not
  found", like opening a deleted page.
- **Refused at save time** (someone logged into the event after the page loaded): the event page
  is shown again with the greyed-out button and the fresh recap.
- **Confirmation works without JavaScript** (server-rendered, e.g. `<details>` with a POST form).

## Docs to update (with Daniele's approval)

- Main spec: deletion of entries/events, the one-open-event rule now enforced by the app.
- `docs/specs/2026-10-19-log-entries-design.md:24,44` ("no delete UI exists").
- New decisions in `docs/DECISIONS.md` from #149 (renumber if `feat/readonly-role` lands
  first; that branch also adds migration 0007, so this one may become 0008).

## Left to the builder

- Route shapes (e.g. `POST /entries/{id}/delete`, `POST /events/{id}/delete`) and how the
  confirmation step is rendered (a `?confirm` query, a `<details>`, or a small confirm page).
- How "event exists" is expressed in queries (join / `EXISTS` on live entries).
- Translation key names.

## Decided during build (2026-11-03, with Daniele)

- **An event needs a live smell report.** When its last report is deleted or moved out, its
  remaining PM2.5 readings become standalone readings (one audited `log_entry` update each),
  like a reading logged with no event running; the event is then gone. Deleting a whole event
  still deletes every entry, readings included.
- **Owner rule:** when the earliest entry leaves and reports remain, the start follows the
  earliest remaining entry (never past the end) and the owner the earliest remaining smell
  report. A reading never makes someone the owner.
- **Move to a new event** on an event's only entry keeps that event and re-centres it; the option
  is hidden there.
- **Guests** see blockers as "altri N", never names, and always the "gli altri" sentence.
- **After deleting an entry:** its event page only if the event still exists and you can still
  open it; otherwise History.
- **Forged requests:** deleting an entry or event you cannot see answers "Page not found"; one
  you can see but may not delete answers "Not allowed".
