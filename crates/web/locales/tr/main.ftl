app-name = Smoke Tracker
close = Kapat
language = Dil

login-title = Giriş yap
login-username = Kullanıcı adı
login-password = Şifre
login-submit = Giriş yap
login-failed = Kullanıcı adı veya şifre geçersiz.
login-locked = Çok fazla deneme. 15 dakika sonra tekrar deneyin.
login-expired = Bu hesabın süresi doldu. Yeniden etkinleştirmesi için bir ev sakinine başvurun.
signed-in-as = { $username } olarak giriş yapıldı.
logout = Çıkış yap

log-title = Koku kaydet
event-none = Devam eden bir koku olayı yok.
event-open-since = { $time } saatinden beri süren bir koku olayı var.
log-intensity = Yoğunluk
log-how-to-rate = Nasıl puanlanır
log-rooms = Odalar
log-level-in = { $room } seviyesi
log-inside = Ev içi
log-common = Ortak alanlar
log-symptoms = Belirtiler
log-pm25 = PM2.5 (µg/m³, isteğe bağlı)
log-time = Saat
log-time-now = şimdi
log-time-change = değiştir
log-note = Not (isteğe bağlı)
log-submit = Kokuyu kaydet
log-update = Güncelle: durum değişti
log-pm25-only = Yalnızca PM2.5 kaydet
log-no-smell = Koku yok
log-read-only = Verilere bakabilirsin ama kayıt yapamazsın.

log-saved-report = Bildirim kaydedildi.
log-saved-pm25 = PM2.5 ölçümü kaydedildi.
log-saved-no-smell = Kaydedildi: koku yok.
log-error-rooms = En az bir oda seç.
log-error-level = Seçtiğin her oda için 0 ile 10 arasında bir seviye seç.
log-error-no-smell-level = Koku yok, seçili odaları 0 olarak kaydeder. 0'dan büyük seviyeleri kaldır ya da Kokuyu kaydet'i kullan.
log-error-pm25 = PM2.5, 0 ile 1000 arasında bir sayı olmalı.
log-error-pm25-missing = Tek başına kaydetmek için bir PM2.5 değeri gir.
log-error-time = Geçerli bir tarih ve saat gir.
log-error-time-skipped = Bu saat yok: saatler ileri alındı.
log-error-note = Not en fazla 2000 karakter olabilir.
log-error-form = Formda geçersiz bir şey var. Kontrol edip tekrar dene.
notice-dismiss = Kapat

prompt-same-smell = Aynı koku hâlâ sürüyor mu?
prompt-ongoing = Devam eden bir koku olayı var ({ $time } saatinde başladı). Bu aynı koku mu?
prompt-yes-update = Evet, güncelle
prompt-no-new-event = Hayır, kesilmişti: yeni olay
prompt-which-event = Bu hangi olaydı?
prompt-started-at = { $time } saatinde başladı
prompt-ended-at = { $time } saatinde bitti
prompt-new-event = Yeni olay
prompt-no-event = Olay yok: yalnızca kontrol
prompt-auto-closed = { $when } tarihli olay otomatik olarak kapatıldı.
prompt-continue = Hâlâ sürüyordu: devamı
prompt-cancel = Vazgeç

refused-stale = Bu arada olaylar değişti. Kontrol edip tekrar dene.
refused-another-open = Açık başka bir olay var. Önce onu kapat.
refused-outside-event = Bu saat olayın dışında: başlamadan önce ya da başka bir günde.
refused-no-smell-new-event = Kokusuz bir bildirim olay başlatamaz.
refused-check-smell = Bu kokusuz bir kontrol, seviyeleri 0 kalır. Bir koku kaydetmek için yeni bir bildirim kaydet.
refused-future = Saat gelecekte olamaz.
refused-close-before-last-report = Bitiş, son bildirimden önce olamaz.
refused-close-in-future = Bitiş gelecekte olamaz.

summary-this-month = Bu ay:
summary-events = { $count ->
    [one] 1 olay
   *[other] { $count } olay
}
summary-reports = { $count ->
    [one] 1 bildirim
   *[other] { $count } bildirim
}
summary-bedroom-days = { $count ->
    [one] yatak odasında duman olan 1 gün
   *[other] yatak odasında duman olan { $count } gün
}
summary-checks = { $count ->
    [one] 1 temiz kontrol
   *[other] { $count } temiz kontrol
}

rating-title = Nasıl puanlanır
rating-level = Seviye
rating-description = Nasıl bir şey
rating-tip = Şu an aldığın kokuyu puanla, ne kadar rahatsız olduğunu değil. İki sayı arasında kalırsan küçük olanı seç.
rating-full-guide = Tüm rehber →

intensity-0-name = Temiz
intensity-0-desc = Kontrol ettin: hiç duman kokusu yok.
intensity-1-name = Zar zor
intensity-1-desc = Ancak dikkat edersen. Duman olduğundan emin değilsin.
intensity-2-name = Hafif
intensity-2-desc = Odaya girerken fark edilir, sonra kaybolur.
intensity-3-name = Az
intensity-3-desc = Açıkça duman, ama zayıf. Görmezden gelmek kolay.
intensity-4-name = Belirgin
intensity-4-desc = Sürekli var, biraz rahatsız edici.
intensity-5-name = Orta
intensity-5-desc = Her an belli. Birine söylerdin.
intensity-6-name = Güçlü
intensity-6-desc = Görmezden gelmek zor; pencereyi açmak ya da odadan çıkmak istersin.
intensity-7-name = Çok güçlü
intensity-7-desc = Rahat duramazsın; koku birikir.
intensity-8-name = Ağır
intensity-8-desc = Giysilere ve yatak takımına siner; çıktıktan sonra da kalır.
intensity-9-name = Çok ağır
intensity-9-desc = Sigara içen birinin yanında durmak gibi.
intensity-10-name = Dayanılmaz
intensity-10-desc = Dumanla dolu bir oda gibi. Çıkmak zorundasın.

nav-log = Kaydet
nav-events = Olaylar
nav-people = Kişiler
nav-settings = Ayarlar

events-title = Olaylar
events-none = Bu ay olay yok.
events-check = koku yok (kontrol)
events-previous = ← Önceki
events-next = Sonraki →
events-in-progress = sürüyor
events-max-intensity = en fazla { $level }
event-title = { $date } tarihli olay
event-started-by = Başlatan: { $username }
event-gap = Önceki bildirimden bu yana 3 saatten fazla geçti: hâlâ aynı olay mı?
event-intensity = yoğunluk { $level }
event-pm25 = PM2.5 { $value } µg/m³
event-close = Olayı kapat
event-close-at = Bitiş saati
event-reopen = Yeniden aç

report-title = Koku bildirimi
report-event = Olay
report-no-event = Yok: kokusuz bir kontrol
report-observed = Gözlem
report-recorded = Kayıt
report-note = Not
report-by = Kaydeden
report-none = yok
report-whole-home = tüm ev
report-edited = düzenlendi
report-deleted = silindi
report-correct = Bir hatayı düzelt
report-move = Başka bir olaya taşı
report-move-submit = Taşı
report-history = Geçmiş
history-create = kaydedildi
history-update = düzeltildi
history-move = taşındı
history-delete = silindi
history-time = saat { $change }
history-added = eklendi: { $name }
history-removed = çıkarıldı: { $name }
history-note = not değişti
history-moved = olay { $change }
correct-hint = Yalnızca hataları düzelt. Durum değiştiyse bunun yerine yeni bir bildirim kaydet.
correct-check-hint = Bu kokusuz bir kontrol: tüm seviyeler 0 kalır. Bir koku aldıysan bunun yerine yeni bir bildirim kaydet.
correct-time = Gözlem zamanı
correct-submit = Düzeltmeyi kaydet

close-reason-manual = elle kapatıldı
close-reason-new-event = yeni bir olay başlayınca kapatıldı
close-reason-auto = gece otomatik olarak kapatıldı
close-reason-backdated = sonradan kaydedildi

settings-title = Ayarlar
settings-password = Şifre
settings-password-current = Mevcut şifre
settings-password-new = Yeni şifre
settings-password-repeat = Yeni şifreyi tekrarlayın
settings-password-hint = 10 ile 128 karakter arası. Diğer cihazlarınızda oturum kapatılacak.
settings-password-submit = Şifreyi değiştir
settings-password-mismatch = İki yeni şifre aynı değil.
settings-password-length = Yeni şifre 10 ile 128 karakter arasında olmalı.
settings-password-wrong = Mevcut şifre doğru değil.
settings-saved-language = Dil kaydedildi.
settings-saved-password = Şifre değiştirildi. Diğer cihazlarınızda oturum kapatıldı.
settings-guides = Kılavuzlar
settings-lists-hint = Gizlenen öğeler artık formlarda sunulmaz; onları içeren kayıtlar korunur.
settings-list-hidden = gizli
settings-list-hide = Gizle
settings-list-show = Yeniden göster
settings-room-add = Yeni oda
settings-symptom-add = Yeni belirti
settings-list-add = Ekle
settings-list-name-invalid = Bir ad 1 ile 60 karakter arasında olmalı.
settings-saved-lists = Listeler kaydedildi.

people-title = Kişiler
role-resident = ev sakini
role-guest = misafir
role-viewer = izleyici
people-status-until = { $when } tarihine kadar
people-status-expired = { $when } tarihinde süresi doldu
people-status-no-end = bitiş tarihi yok
people-status-setup = kurulum bekleniyor
people-expire = Erişimi şimdi sonlandır
people-reactivate = Yeniden etkinleştir
people-saved-expired = Erişim sonlandırıldı. Tüm cihazlarda oturumu kapatıldı.
people-saved-reactivated = Hesap yeniden etkinleştirildi. Eski şifre hâlâ geçerli.
people-error-expiry = Misafirin ne kadar kalabileceğini seçin.
people-error-date = Geçerli bir tarih girin.
people-error-past = Bitiş gelecekte olmalı.
people-error-not-expired = Bu hesabın süresi dolmadı.
people-error-resident = Ev sakinlerinin süresi hiç dolmaz.
expiry-legend = Erişimin bitişi
expiry-1-day = 1 gün sonra
expiry-weekend = hafta sonundan sonra
expiry-1-week = 1 hafta sonra
expiry-2-weeks = 2 hafta sonra
expiry-date = bu günün sonunda:
expiry-never = hiçbir zaman

error-not-found-title = Sayfa bulunamadı
error-not-found = Bu sayfa mevcut değil.
error-forbidden-title = İstek engellendi
error-forbidden = Bu istek bu uygulamadan gelmediği için engellendi. Geri dönüp tekrar deneyin.
error-not-allowed-title = İzin yok
error-not-allowed = Hesabın bunu yapamaz.
error-internal-title = Bir şeyler ters gitti
error-internal = Bir şeyler ters gitti (ref: { $ref }).
error-back-home = Başa dön

room-living-room = Oturma odası
room-kitchen = Mutfak
room-bedroom = Yatak odası
room-bathroom = Banyo
room-entrance-hall = Antre
room-stairwell = Merdiven boşluğu (ortak)

symptom-cough = Öksürük
symptom-headache = Baş ağrısı
symptom-sore-throat = Boğaz ağrısı
symptom-eye-irritation = Göz tahrişi
symptom-nausea = Mide bulantısı
symptom-difficulty-sleeping = Uyku güçlüğü
