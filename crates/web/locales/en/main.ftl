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

error-not-found-title = Page not found
error-not-found = This page does not exist.
error-forbidden-title = Request blocked
error-forbidden = This request did not come from this app, so it was blocked. Go back and try again.
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
