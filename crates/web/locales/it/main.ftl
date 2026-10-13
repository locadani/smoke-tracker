app-name = Smoke Tracker
close = Chiudi
language = Lingua

login-title = Accedi
login-username = Nome utente
login-password = Password
login-submit = Accedi
login-failed = Nome utente o password non validi.
login-locked = Troppi tentativi. Riprova tra 15 minuti.
login-expired = Questo account è scaduto. Chiedi a un residente di riattivarlo.
signed-in-as = Accesso effettuato come { $username }.
logout = Esci

log-title = Registra un odore
event-none = Nessun evento di fumo in corso.
event-open-since = Evento di fumo in corso dalle { $time }.
log-intensity = Intensità
log-how-to-rate = Come valutare
log-rooms = Stanze
log-level-in = Livello in { $room }
log-inside = In casa
log-common = Parti comuni
log-symptoms = Sintomi
log-pm25 = PM2.5 (µg/m³, facoltativo)
log-time = Ora
log-time-now = adesso
log-time-change = cambia
log-note = Nota (facoltativa)
log-submit = Registra odore
log-update = Aggiorna: la situazione è cambiata
log-pm25-only = Registra solo PM2.5
log-no-smell = Nessun odore
log-read-only = Puoi consultare i dati ma non registrare.

log-saved-report = Segnalazione salvata.
log-saved-pm25 = Lettura PM2.5 salvata.
log-saved-no-smell = Salvato: nessun odore.
log-error-rooms = Scegli almeno una stanza.
log-error-level = Scegli un livello da 0 a 10 per ogni stanza selezionata.
log-error-no-smell-level = Nessun odore registra le stanze selezionate a 0. Togli i livelli sopra 0, oppure usa Registra odore.
log-error-pm25 = Il PM2.5 deve essere un numero da 0 a 1000.
log-error-pm25-missing = Inserisci un valore di PM2.5 per registrarlo da solo.
log-error-time = Inserisci una data e un'ora valide.
log-error-time-skipped = Quell'ora non esiste: è scattata l'ora legale.
log-error-note = La nota può avere al massimo 2000 caratteri.
log-error-form = Qualcosa nel modulo non è valido. Controlla e riprova.
notice-dismiss = Ignora

prompt-same-smell = È ancora lo stesso odore?
prompt-ongoing = C'è un evento in corso (iniziato alle { $time }). È lo stesso odore?
prompt-yes-update = Sì, aggiorna
prompt-no-new-event = No, era sparito: nuovo evento
prompt-which-event = Di quale evento si trattava?
prompt-started-at = Iniziato alle { $time }
prompt-ended-at = finito alle { $time }
prompt-new-event = Nuovo evento
prompt-no-event = Nessun evento: solo un controllo
prompt-auto-closed = L'evento del { $when } è stato chiuso automaticamente.
prompt-continue = Continuava ancora: è lo stesso evento
prompt-cancel = Annulla

refused-stale = Nel frattempo gli eventi sono cambiati. Controlla e riprova.
refused-another-open = C'è un altro evento aperto. Chiudilo prima.
refused-outside-event = Quell'ora è fuori dall'evento: prima che iniziasse o in un altro giorno.
refused-no-smell-new-event = Una segnalazione senza odore non può iniziare un evento.
refused-check-smell = Questo è un controllo senza odore: i livelli restano a 0. Per registrare un odore, registra una nuova segnalazione.
refused-future = L'ora non può essere nel futuro.
refused-close-before-last-report = La fine non può essere prima dell'ultima segnalazione.
refused-close-in-future = La fine non può essere nel futuro.

summary-this-month = Questo mese:
summary-events = { $count ->
    [one] 1 evento
   *[other] { $count } eventi
}
summary-reports = { $count ->
    [one] 1 segnalazione
   *[other] { $count } segnalazioni
}
summary-bedroom-days = { $count ->
    [one] 1 giorno con fumo in camera da letto
   *[other] { $count } giorni con fumo in camera da letto
}
summary-checks = { $count ->
    [one] 1 controllo pulito
   *[other] { $count } controlli puliti
}

rating-title = Come valutare
rating-level = Livello
rating-description = Come si presenta
rating-tip = Valuta quello che senti adesso, non quanto sei infastidito. Tra due numeri, scegli il più basso.
rating-full-guide = Guida completa →

intensity-0-name = Pulito
intensity-0-desc = Hai controllato: nessun odore di fumo.
intensity-1-name = Appena percepibile
intensity-1-desc = Solo se ci fai caso. Non sei sicuro che sia fumo.
intensity-2-name = Debole
intensity-2-desc = Si nota entrando nella stanza, poi svanisce.
intensity-3-name = Leggero
intensity-3-desc = Chiaramente fumo, ma debole. Facile da ignorare.
intensity-4-name = Percepibile
intensity-4-desc = Sempre presente, un po' fastidioso.
intensity-5-name = Moderato
intensity-5-desc = Evidente tutto il tempo. Lo faresti notare.
intensity-6-name = Forte
intensity-6-desc = Difficile da ignorare; vorresti aprire una finestra o uscire dalla stanza.
intensity-7-name = Molto forte
intensity-7-desc = Non si riesce a stare comodi; l'odore si accumula.
intensity-8-name = Pesante
intensity-8-desc = Entra nei vestiti e nelle lenzuola; resta anche dopo che sei uscito.
intensity-9-name = Molto pesante
intensity-9-desc = Come stare accanto a qualcuno che fuma.
intensity-10-name = Insopportabile
intensity-10-desc = Come una stanza piena di fumo. Devi uscire.

nav-log = Registra
nav-events = Eventi
nav-people = Persone
nav-settings = Impostazioni

events-title = Eventi
events-none = Nessun evento questo mese.
events-check = nessun odore (controllo)
events-previous = ← Precedente
events-next = Successivo →
events-in-progress = in corso
events-max-intensity = max { $level }
event-title = Evento del { $date }
event-started-by = Iniziato da { $username }
event-gap = Più di 3 ore dalla segnalazione precedente: è ancora lo stesso evento?
event-intensity = intensità { $level }
event-pm25 = PM2.5 { $value } µg/m³
event-close = Chiudi evento
event-close-at = Ora di fine
event-reopen = Riapri

report-title = Segnalazione
report-event = Evento
report-no-event = Nessuno: un controllo senza odore
report-observed = Osservata
report-recorded = Registrata
report-note = Nota
report-by = Registrata da
report-none = nessuno
report-whole-home = tutta la casa
report-edited = modificata
report-deleted = eliminata
report-correct = Correggi un errore
report-move = Sposta in un altro evento
report-move-submit = Sposta
report-history = Cronologia
history-create = registrata
history-update = corretta
history-move = spostata
history-delete = eliminata
history-time = ora { $change }
history-added = aggiunto { $name }
history-removed = tolto { $name }
history-note = nota modificata
history-moved = evento { $change }
correct-hint = Correggi solo gli errori. Se la situazione è cambiata, registra invece una nuova segnalazione.
correct-check-hint = Questo è un controllo senza odore: ogni livello resta a 0. Se hai sentito un odore, registra invece una nuova segnalazione.
correct-time = Osservata alle
correct-submit = Salva la correzione

close-reason-manual = chiuso a mano
close-reason-new-event = chiuso all'inizio di un nuovo evento
close-reason-auto = chiuso automaticamente durante la notte
close-reason-backdated = registrato in seguito

settings-title = Impostazioni
settings-password = Password
settings-password-current = Password attuale
settings-password-new = Nuova password
settings-password-repeat = Ripeti la nuova password
settings-password-hint = Da 10 a 128 caratteri. Gli altri dispositivi verranno disconnessi.
settings-password-submit = Cambia password
settings-password-mismatch = Le due nuove password non coincidono.
settings-password-length = La nuova password deve avere da 10 a 128 caratteri.
settings-password-wrong = La password attuale non è corretta.
settings-saved-language = Lingua salvata.
settings-saved-password = Password cambiata. Gli altri dispositivi sono stati disconnessi.
settings-guides = Guide
settings-lists-hint = Le voci nascoste non sono più proposte nei moduli; le segnalazioni che le citano le mantengono.
settings-list-hidden = nascosto
settings-list-hide = Nascondi
settings-list-show = Mostra di nuovo
settings-room-add = Nuova stanza
settings-symptom-add = Nuovo sintomo
settings-list-add = Aggiungi
settings-list-name-invalid = Un nome deve avere da 1 a 60 caratteri.
settings-saved-lists = Elenchi salvati.

people-title = Persone
role-resident = residente
role-guest = ospite
role-viewer = osservatore
people-status-until = fino al { $when }
people-status-expired = scaduto il { $when }
people-status-no-end = senza scadenza
people-status-setup = in attesa di configurazione
people-expire = Termina l'accesso ora
people-reactivate = Riattiva
people-saved-expired = Accesso terminato. La persona è stata disconnessa ovunque.
people-saved-reactivated = Account riattivato. La vecchia password funziona ancora.
people-error-expiry = Scegli quanto può restare l'ospite.
people-error-date = Inserisci una data valida.
people-error-past = La scadenza deve essere nel futuro.
people-error-not-expired = Questo account non è scaduto.
people-error-resident = I residenti non scadono mai.
people-error-username = Un nome utente deve avere da 3 a 32 caratteri: lettere, cifre, ".", "_" o "-".
people-error-username-taken = Questo nome utente è già in uso.
people-invite = Invita qualcuno
people-new-setup-link = Nuovo codice QR di configurazione
people-reset-password = Reimposta la password
invite-title = Invita qualcuno
invite-username-hint = Accederà con questo nome. Lettere, cifre, ".", "_" o "-".
invite-role = Ruolo
invite-submit = Crea il codice QR
link-title = Codice QR
link-setup-title = Link di configurazione per { $username }
link-reset-title = Reimpostazione password per { $username }
link-scan = Fai inquadrare questo codice con la fotocamera del telefono.
link-valid-until = Funziona una sola volta, fino alle { $time } (5 minuti).
link-text = Oppure invia il link:
link-done = Fatto
accept-title = Scegli una nuova password
accept-welcome = Benvenuto, { $username }
accept-setup-intro = Scegli password e lingua per completare il tuo account.
accept-reset-intro = Scegli una nuova password per { $username }. Tutti i dispositivi collegati a questo account verranno disconnessi.
accept-password-hint = Da 10 a 128 caratteri.
accept-submit = Salva e accedi
expiry-legend = L'accesso termina
expiry-1-day = tra 1 giorno
expiry-weekend = dopo il fine settimana
expiry-1-week = tra 1 settimana
expiry-2-weeks = tra 2 settimane
expiry-date = alla fine di questo giorno:
expiry-never = mai

install-app = Installa l'app
install-title = Aggiungi Smoke Tracker alla schermata Home
install-welcome = Il tuo account è pronto.
install-intro = Si aprirà come un'app, direttamente sulla schermata Registra: registrare richiede pochi secondi.
install-button = Installa
install-ios-title = iPhone e iPad (Safari)
install-ios-share = Tocca il pulsante Condividi
install-ios-add = Scegli "Aggiungi alla schermata Home"
install-ios-confirm = Tocca "Aggiungi".
install-other = Su Android o sul computer, se non compare il pulsante Installa, apri il menu del browser e scegli "Installa app" o "Aggiungi a schermata Home".
install-continue = Vai all'app
offline-title = Nessuna connessione
offline-text = Smoke Tracker ha bisogno di internet per salvare ciò che registri. Riprova quando sei di nuovo online.
offline-retry = Riprova

error-not-found-title = Pagina non trovata
error-not-found = Questa pagina non esiste.
error-forbidden-title = Richiesta bloccata
error-forbidden = Questa richiesta non proviene da questa app, quindi è stata bloccata. Torna indietro e riprova.
error-not-allowed-title = Non consentito
error-not-allowed = Il tuo account non può farlo.
error-internal-title = Qualcosa è andato storto
error-internal = Qualcosa è andato storto (rif: { $ref }).
error-link-title = Questo link non funziona più
error-link = I link funzionano una sola volta e per 5 minuti. Chiedi un nuovo codice QR a un residente.
error-back-home = Torna all'inizio

room-living-room = Soggiorno
room-kitchen = Cucina
room-bedroom = Camera da letto
room-bathroom = Bagno
room-entrance-hall = Ingresso
room-stairwell = Vano scale (condominiale)

symptom-cough = Tosse
symptom-headache = Mal di testa
symptom-sore-throat = Mal di gola
symptom-eye-irritation = Irritazione agli occhi
symptom-nausea = Nausea
symptom-difficulty-sleeping = Difficoltà a dormire
