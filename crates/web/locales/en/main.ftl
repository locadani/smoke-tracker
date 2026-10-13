app-name = Smoke Tracker
close = Close
language = Language

login-title = Sign in
login-username = Username
login-password = Password
login-submit = Sign in
login-failed = Invalid username or password.
login-locked = Too many attempts. Try again in 15 minutes.
login-expired = This account has expired. Ask a resident to reactivate it.
signed-in-as = Signed in as { $username }.
logout = Sign out

log-title = Log a smell
event-none = No smell event in progress.
event-open-since = Smell event in progress since { $time }.
log-intensity = Intensity
log-how-to-rate = How to rate
log-rooms = Rooms
log-inside = Inside
log-common = Common areas
log-symptoms = Symptoms
log-pm25 = PM2.5 (µg/m³, optional)
log-time = Time
log-time-now = now
log-time-change = change
log-note = Note (optional)
log-submit = Log smell
log-update = Update: situation changed
log-pm25-only = Log PM2.5 only
log-read-only = You can look at the data but not log.

log-saved-report = Report saved.
log-saved-pm25 = PM2.5 reading saved.
log-error-intensity = Pick an intensity from 1 to 10.
log-error-pm25 = PM2.5 must be a number from 0 to 1000.
log-error-pm25-missing = Enter a PM2.5 value to log it on its own.
log-error-time = Enter a valid date and time.
log-error-time-skipped = That time does not exist: the clocks moved forward.
log-error-note = The note can be at most 2000 characters.
log-error-form = Something in the form is not valid. Check it and try again.
notice-dismiss = Dismiss

prompt-same-smell = Same smell still going?
prompt-ongoing = A smell event is ongoing (started { $time }). Is this the same smell?
prompt-yes-update = Yes, update
prompt-no-new-event = No, it had stopped: new event
prompt-which-event = Which event was this?
prompt-started-at = Started { $time }
prompt-ended-at = ended { $time }
prompt-new-event = New event
prompt-auto-closed = The event from { $when } was auto-closed.
prompt-continue = It was still going: continuation
prompt-cancel = Cancel

refused-stale = The events changed in the meantime. Check and try again.
refused-another-open = Another event is open. Close it first.
refused-outside-event = That time is outside the event: before it started, or on another day.
refused-future = The time cannot be in the future.
refused-close-before-last-report = The end cannot be before the last report.
refused-close-in-future = The end cannot be in the future.

summary-this-month = This month:
summary-events = { $count ->
    [one] 1 event
   *[other] { $count } events
}
summary-reports = { $count ->
    [one] 1 report
   *[other] { $count } reports
}
summary-bedroom-days = { $count ->
    [one] 1 day with bedroom smoke
   *[other] { $count } days with bedroom smoke
}

rating-title = How to rate
rating-level = Level
rating-description = What it's like
rating-tip = Rate what you smell right now, not how annoyed you are. Between two numbers, pick the lower.
rating-full-guide = Full guide →

intensity-1-name = Barely there
intensity-1-desc = Only if you focus. Unsure it's smoke.
intensity-2-name = Faint
intensity-2-desc = Noticeable entering the room, then fades.
intensity-3-name = Light
intensity-3-desc = Clearly smoke, but weak. Easy to ignore.
intensity-4-name = Noticeable
intensity-4-desc = Constantly present, mildly annoying.
intensity-5-name = Moderate
intensity-5-desc = Obvious all the time. You'd mention it.
intensity-6-name = Strong
intensity-6-desc = Hard to ignore; want to open a window or leave the room.
intensity-7-name = Very strong
intensity-7-desc = Can't stay comfortably; smell builds up.
intensity-8-name = Heavy
intensity-8-desc = Gets into clothes/bedding; lingers after you leave.
intensity-9-name = Very heavy
intensity-9-desc = Like standing next to someone smoking.
intensity-10-name = Unbearable
intensity-10-desc = Like a smoke-filled room. You must leave.

nav-log = Log
nav-events = Events

events-title = Events
events-none = No events this month.
events-previous = ← Previous
events-next = Next →
events-in-progress = in progress
events-max-intensity = max { $level }
event-title = Event of { $date }
event-started-by = Started by { $username }
event-gap = More than 3 hours since the previous report: is this still the same event?
event-intensity = intensity { $level }
event-pm25 = PM2.5 { $value } µg/m³
event-close = Close event
event-close-at = End time
event-reopen = Reopen

report-title = Smell report
report-event = Event
report-observed = Observed
report-recorded = Recorded
report-note = Note
report-by = Logged by
report-none = none
report-edited = edited
report-deleted = deleted
report-correct = Correct a mistake
report-move = Move to another event
report-move-submit = Move
report-history = History
history-create = logged
history-update = corrected
history-move = moved
history-delete = deleted
history-time = time { $change }
history-intensity = intensity { $change }
history-added = added { $name }
history-removed = removed { $name }
history-note = note changed
history-moved = event { $change }
correct-hint = Correct only mistakes. If the situation changed, log a new report instead.
correct-time = Observed at
correct-submit = Save the correction

close-reason-manual = closed by hand
close-reason-new-event = closed when a new event started
close-reason-auto = closed automatically overnight
close-reason-backdated = logged afterwards

error-not-found-title = Page not found
error-not-found = This page does not exist.
error-forbidden-title = Request blocked
error-forbidden = This request did not come from this app, so it was blocked. Go back and try again.
error-not-allowed-title = Not allowed
error-not-allowed = Your account cannot do this.
error-internal-title = Something went wrong
error-internal = Something went wrong (ref: { $ref }).
error-back-home = Back to the start

room-living-room = Living room
room-kitchen = Kitchen
room-bedroom = Bedroom
room-bathroom = Bathroom
room-entrance-hall = Entrance hall
room-stairwell = Stairwell (shared)

symptom-cough = Cough
symptom-headache = Headache
symptom-sore-throat = Sore throat
symptom-eye-irritation = Eye irritation
symptom-nausea = Nausea
symptom-difficulty-sleeping = Difficulty sleeping
