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
account-menu = Account: { $username }

log-title = Log a smell
event-banner-title = Smell event in progress
event-banner-since = since { $time }
event-banner-peak = highest level { $level } ({ $room })
log-intensity = Intensity
log-how-to-rate = How to rate
log-rooms = Rooms
log-level-in = Level in { $room }
log-inside = Inside
log-common = Common areas
log-symptoms = Symptoms
log-time = Time
log-time-now = now
log-note = Note
log-submit = Log
log-update = Update: situation changed
log-no-smell = No smell
log-read-only = You can look at the data but not log.

log-saved-report = Report saved.
log-saved-pm25 = PM2.5 reading saved.
log-saved-no-smell = Saved: no smell.
log-error-rooms = Pick at least one room.
log-error-level = Pick a level from 0 to 10 for each selected room.
log-error-no-smell-level = No smell records the selected rooms at 0. Remove the levels above 0, or use Log.
log-error-pm25 = PM2.5 must be a number from 0 to 1000.
log-error-empty = Pick a room, or enter a PM2.5 value.
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
prompt-no-event = No event: just a check
prompt-auto-closed = The event from { $when } was auto-closed.
prompt-continue = It was still going: continuation
prompt-cancel = Cancel

refused-stale = The events changed in the meantime. Check and try again.
refused-another-open = Another event is open. Close it first.
refused-outside-event = That time is outside the event: before it started, or on another day.
refused-no-smell-new-event = A report with no smell cannot start an event.
refused-check-smell = This is a no-smell check, so its levels stay at 0. To record a smell, log a new report.
refused-future = The time cannot be in the future.
refused-close-before-last-report = The end cannot be before the last report.
refused-close-in-future = The end cannot be in the future.

summary-reports = { $count ->
    [one] 1 report
   *[other] { $count } reports
}
log-level-hint = Pick a level from 0 to 10.
log-rooms-hint = Tick only the rooms you checked now. Unticked rooms count as "not checked", not clean. Clean = 0.
log-rooms-hint-prefilled = Rooms from your last report are already ticked: untick the ones you didn't check now. Unticked rooms count as "not checked", not clean. Clean = 0.
optional = optional
action-add = Add
action-change = Change
summary-caption = This month
tile-events = { $count ->
    [one] event
   *[other] events
}
tile-reports = { $count ->
    [one] report
   *[other] reports
}
tile-bedroom-days = { $count ->
    [one] bedroom day
   *[other] bedroom days
}
tile-checks = { $count ->
    [one] clean check
   *[other] clean checks
}

rating-title = How to rate
rating-level = Level
rating-description = What it's like
rating-tip = Rate what you smell right now, not how annoyed you are. Between two numbers, pick the lower.
rating-full-guide = Full guide →

intensity-0-name = Clean
intensity-0-desc = You checked: no smoke smell at all.
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

guide-log-title = How to log
guide-log-happened = What happened
guide-log-action = Correct action
guide-log-mistake = I made a mistake (wrong number, forgot a room, wrong symptom)
guide-log-mistake-action = Correct that report (edit; recorded in history)
guide-log-changed = The situation changed (stronger/weaker, a room started or stopped, symptoms started)
guide-log-changed-action = Add a new report to the same event
guide-log-gone = The smell went away completely and came back later
guide-log-gone-action = Start a new event
guide-log-rules = Rules of thumb
guide-log-rule-1 = Never edit to show a change; add a report. Editing changes the past; adding records the present.
guide-log-rule-2 = Add a report when a room's level changes by 2 or more, a room starts or clears (log it at 0), or a new symptom appears. Each report lists only the rooms you checked: a room you leave out is "not checked", never "clean".
guide-log-rule-3 = Unsure whether it's the same event? If the smell was completely gone at some point, it's a new one.
guide-log-rule-4 = Close the event when the smell is gone. Forgetting is fine ({ $hour } auto-close), but closing gives accurate durations.
guide-log-rule-5 = Nothing smells? Log No smell. It keeps a record that you checked.
guide-log-sensitivity-title = Different sensitivities
guide-log-sensitivity = Each person rates on their own 1–10 scale. Overlapping reports with different values are kept as-is. Graphs show one line per person; the PM2.5 reading is the objective reference.

nav-log = Log
nav-events = Events
nav-graphs = Graphs
nav-people = People
nav-settings = Settings
nav-label = Main menu

events-title = Events
events-none = No events this month.
events-check = no smell (check)
events-previous = Previous month
events-next = Next month
events-month = Month
events-in-progress = in progress
events-max-intensity = max { $level }
event-title = Event of { $date }
event-started-by = Started by { $username }
event-timeline = Timeline
event-timeline-hint = Each report shows only the rooms checked at that time.
event-close-title = Close this event
event-gap = More than 3 hours since the previous report: is this still the same event?
event-gap-move = Move { $time } to a new event
event-intensity = intensity { $level }
event-pm25 = PM2.5 { $value } µg/m³
event-close = Close event
event-close-at = End time
event-reopen = Reopen

report-title = Smell report
report-event = Event
report-no-event = None: a no-smell check
report-observed = Observed
report-recorded = Recorded
report-note = Note
report-by = Logged by
report-none = none
report-whole-home = whole home
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
history-added = added { $name }
history-removed = removed { $name }
history-note = note changed
history-moved = event { $change }
correct-hint = Correct only mistakes. If the situation changed, log a new report instead.
correct-check-hint = This is a no-smell check: every level stays at 0. If you smelled something, log a new report instead.
correct-time = Observed at
correct-submit = Save the correction

close-reason-manual = closed by hand
close-reason-new-event = closed when a new event started
close-reason-auto = closed automatically overnight
close-reason-backdated = logged afterwards

settings-title = Settings
settings-password = Password
settings-password-current = Current password
settings-password-new = New password
settings-password-repeat = Repeat the new password
settings-password-hint = 10 to 128 characters. Your other devices will be signed out.
settings-password-submit = Change password
settings-password-mismatch = The two new passwords do not match.
settings-password-length = The new password needs 10 to 128 characters.
settings-password-wrong = The current password is not correct.
settings-saved-language = Language saved.
settings-saved-password = Password changed. Your other devices are signed out.
settings-guides = Guides
settings-room-area = Area
settings-lists-hint = Hidden items are no longer offered on the forms; reports that name them keep them.
settings-list-hidden = hidden
settings-list-hide = Hide
settings-list-show = Show again
settings-room-add = New room
settings-symptom-add = New symptom
settings-list-add = Add
settings-list-name-invalid = A name needs 1 to 60 characters.
settings-saved-lists = Lists saved.

people-title = People
role-resident = resident
role-guest = guest
role-viewer = viewer
people-status-until = until { $when }
people-status-expired = expired { $when }
people-status-no-end = no end date
people-status-setup = waiting for setup
people-expire = End access now
people-reactivate = Reactivate
people-saved-expired = Access ended. They are signed out everywhere.
people-saved-reactivated = Account reactivated. The old password still works.
people-error-expiry = Pick how long a guest may stay.
people-error-date = Enter a valid date.
people-error-date-pick = Choose “at the end of this day” for that date, or clear the date.
people-error-past = The end must be in the future.
people-error-not-expired = This account has not expired.
people-error-resident = Residents never expire.
people-error-username = A username needs 3 to 32 characters: letters, digits, ".", "_" or "-".
people-error-username-taken = That username is taken.
people-invite = Invite someone
people-new-setup-link = New setup QR code
people-reset-password = Reset password
invite-title = Invite someone
invite-username-hint = They sign in with this name. Letters, digits, ".", "_" or "-".
invite-role = Role
invite-submit = Create the QR code
link-title = QR code
link-setup-title = Setup link for { $username }
link-reset-title = Password reset for { $username }
link-scan = Let them scan this code with their phone's camera.
link-valid-until = It works once, until { $time } (5 minutes).
link-text = Or send them the link:
link-done = Done
accept-title = Choose a new password
accept-welcome = Welcome, { $username }
accept-setup-intro = Choose your password and language to finish setting up your account.
accept-reset-intro = Choose a new password for { $username }. Every device signed in to this account is signed out.
accept-password-hint = 10 to 128 characters.
accept-submit = Save and sign in
expiry-legend = Access ends
expiry-1-day = in 1 day
expiry-weekend = after the weekend
expiry-1-week = in 1 week
expiry-2-weeks = in 2 weeks
expiry-date = at the end of this day:
expiry-never = never

install-app = Install app
install-card-hint = Open it from your home screen, like any app.
install-title = Add Smoke Tracker to your home screen
install-welcome = Your account is ready.
install-intro = It then opens like an app, straight on the Log screen, so logging takes a few seconds.
install-button = Install
install-ios-title = iPhone and iPad (Safari)
install-ios-share = Tap the Share button
install-ios-add = Choose "Add to Home Screen"
install-ios-confirm = Tap "Add".
install-other = On Android or a computer, if no Install button appears, open the browser menu and choose "Install app" or "Add to Home screen".
install-continue = Continue to the app
offline-title = No connection
offline-text = Smoke Tracker needs the internet to save what you log. Try again when you are back online.
offline-retry = Try again

graphs-title = Graphs
graphs-filters = Filters
graphs-dates = Dates
graphs-range-month = This month
graphs-range-7d = Last 7 days
graphs-range-30d = Last 30 days
graphs-range-year = This year
graphs-from = From
graphs-to = To
graphs-where = Where
graphs-area-all = All
graphs-by = Reported by
graphs-everyone = Everyone
graphs-time = Time of day
graphs-time-any = Any
graphs-time-morning = Morning
graphs-time-afternoon = Afternoon
graphs-time-evening = Evening
graphs-time-night = Night
graphs-min-level = Minimum level
graphs-level-any = any
graphs-level-from = level { $level }+
graphs-hide-clean = Hide clean checks
graphs-no-clean = no clean checks
graphs-reset = Reset
graphs-show = Show graphs
graphs-own-only = These graphs show only your own reports and readings.
graphs-none = Nothing was logged in this period.
graphs-needs-js = This graph needs JavaScript.
graphs-intensity = Smell intensity over time
graphs-pm25 = PM2.5 (µg/m³)
graphs-series-both = Both
graphs-series-intensity = Intensity
graphs-series-pm25 = PM2.5
graphs-series-both-title = Smell intensity and PM2.5
graphs-series-pm25-title = PM2.5 over time
graphs-lines-person = By person
graphs-search-people = Search people
graphs-events-day = Events per day
graphs-events-week = Events per week
graphs-events-label = Events
graphs-rooms = Rooms by hour of day
graphs-rooms-hint = The average level each room was given at each hour, 0 meaning checked and clean. An empty cell means nobody checked that room at that hour.
graphs-rooms-none = No room was checked in this period.
graphs-room = Room
graphs-symptoms-label = Reports
graphs-symptoms-none = No symptoms were reported in this period.
graphs-areas = Inside vs common areas
graphs-areas-hint = Each bar is the highest level reported in that day or week.

error-not-found-title = Page not found
error-not-found = This page does not exist.
error-forbidden-title = Request blocked
error-forbidden = This request did not come from this app, so it was blocked. Go back and try again.
error-not-allowed-title = Not allowed
error-not-allowed = Your account cannot do this.
error-internal-title = Something went wrong
error-internal = Something went wrong (ref: { $ref }).
error-link-title = This link no longer works
error-link = Links work once and for 5 minutes. Ask a resident for a new QR code.
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
