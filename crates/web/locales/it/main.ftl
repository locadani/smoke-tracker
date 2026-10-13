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

error-not-found-title = Pagina non trovata
error-not-found = Questa pagina non esiste.
error-forbidden-title = Richiesta bloccata
error-forbidden = Questa richiesta non proviene da questa app, quindi è stata bloccata. Torna indietro e riprova.
error-not-allowed-title = Non consentito
error-not-allowed = Il tuo account non può farlo.
error-internal-title = Qualcosa è andato storto
error-internal = Qualcosa è andato storto (rif: { $ref }).
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
