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
account-menu = Hesap: { $username }

log-title = Koku kaydet
event-banner-title = Koku olayı sürüyor
event-banner-since = { $time } itibarıyla
event-banner-peak = en yüksek seviye { $level } ({ $room })
log-intensity = Yoğunluk
log-how-to-rate = Nasıl puanlanır
log-rooms = Odalar
log-level-in = { $room } seviyesi
log-inside = Ev içi
log-common = Ortak alanlar
log-symptoms = Belirtiler
log-time = Saat
log-time-now = şimdi
log-note = Not
log-submit = Kaydet
log-update = Güncelle: durum değişti
log-no-smell = Koku yok
log-read-only = Verilere bakabilirsin ama kayıt yapamazsın.

log-saved-report = Bildirim kaydedildi.
log-saved-pm25 = PM2.5 ölçümü kaydedildi.
log-saved-no-smell = Kaydedildi: koku yok.
log-error-rooms = En az bir oda seç.
log-error-level = Seçtiğin her oda için 0 ile 10 arasında bir seviye seç.
log-error-no-smell-level = Koku yok, seçili odaları 0 olarak kaydeder. 0'dan büyük seviyeleri kaldır ya da Kaydet'i kullan.
log-error-pm25 = PM2.5, 0 ile 1000 arasında bir sayı olmalı.
log-error-empty = Bir oda seç ya da bir PM2.5 değeri gir.
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

summary-reports = { $count ->
    [one] 1 bildirim
   *[other] { $count } bildirim
}
log-level-hint = 0 ile 10 arasında bir seviye seç.
log-rooms-hint = Yalnızca şimdi kontrol ettiğin odaları işaretle. İşaretlenmeyen odalar temiz değil, "kontrol edilmedi" sayılır. Temiz = 0.
log-rooms-hint-prefilled = Son raporundaki odalar zaten işaretli: şimdi kontrol etmediklerinin işaretini kaldır. İşaretlenmeyen odalar temiz değil, "kontrol edilmedi" sayılır. Temiz = 0.
optional = isteğe bağlı
action-add = Ekle
action-change = Değiştir
summary-caption = Bu ay
tile-events = { $count ->
    [one] olay
   *[other] olay
}
tile-reports = { $count ->
    [one] bildirim
   *[other] bildirim
}
tile-bedroom-days = { $count ->
    [one] yatak odasında dumanlı gün
   *[other] yatak odasında dumanlı gün
}
tile-checks = { $count ->
    [one] temiz kontrol
   *[other] temiz kontrol
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

guide-log-title = Nasıl kaydedilir
guide-log-happened = Ne oldu
guide-log-action = Doğru adım
guide-log-mistake = Bir hata yaptım (yanlış sayı, unutulan oda, yanlış belirti)
guide-log-mistake-action = O bildirimi düzelt (düzenleme; geçmişte kayıtlı kalır)
guide-log-changed = Durum değişti (daha güçlü/daha zayıf, bir odada başladı ya da bitti, belirtiler başladı)
guide-log-changed-action = Aynı olaya yeni bir bildirim ekle
guide-log-gone = Koku tamamen geçti ve daha sonra geri geldi
guide-log-gone-action = Yeni bir olay başlat
guide-log-rules = Temel kurallar
guide-log-rule-1 = Bir değişikliği göstermek için asla düzenleme yapma; bildirim ekle. Düzenlemek geçmişi değiştirir; eklemek şimdiyi kaydeder.
guide-log-rule-2 = Bir odanın seviyesi 2 veya daha fazla değiştiğinde, bir odada koku başladığında ya da geçtiğinde (0 olarak kaydet) veya yeni bir belirti ortaya çıktığında bildirim ekle. Her bildirim yalnızca kontrol ettiğin odaları listeler: eklemediğin bir oda "kontrol edilmedi" demektir, asla "temiz" değil.
guide-log-rule-3 = Aynı olay olup olmadığından emin değil misin? Koku bir noktada tamamen geçtiyse, bu yeni bir olaydır.
guide-log-rule-4 = Koku geçince olayı kapat. Unutman sorun değil ({ $hour } otomatik kapanış), ama kapatmak süreleri doğru tutar.
guide-log-rule-5 = Koku yok mu? Koku yok olarak kaydet. Kontrol ettiğinin kaydı kalır.
guide-log-sensitivity-title = Farklı hassasiyetler
guide-log-sensitivity = Herkes kendi 1–10 ölçeğine göre puanlar. Farklı değerler içeren çakışan bildirimler olduğu gibi saklanır. Grafikler kişi başına bir çizgi gösterir; PM2.5 ölçümü nesnel referanstır.

nav-log = Kaydet
nav-events = Olaylar
nav-graphs = Grafikler
nav-people = Kişiler
nav-settings = Ayarlar
nav-label = Ana menü

events-title = Olaylar
events-none = Bu ay olay yok.
events-check = koku yok (kontrol)
events-previous = Önceki ay
events-next = Sonraki ay
events-month = Ay
events-in-progress = sürüyor
events-max-intensity = en fazla { $level }
event-title = { $date } tarihli olay
event-started-by = Başlatan: { $username }
event-timeline = Zaman çizelgesi
event-timeline-hint = Her rapor yalnızca o anda kontrol edilen odaları gösterir.
event-close-title = Bu olayı kapat
event-gap = Önceki bildirimden bu yana 3 saatten fazla geçti: hâlâ aynı olay mı?
event-gap-move = { $time } kaydını yeni bir olaya taşı
event-intensity = yoğunluk { $level }
event-pm25 = PM2.5 { $value } µg/m³
event-close = Olayı kapat
event-close-at = Bitiş saati
event-reopen = Yeniden aç

entry-title = Kayıt
entry-pm25-label = PM2.5
entry-pm25-value = { $value } µg/m³
entry-note-label = Not
entry-no-edit-for-reading = Yalnızca bir PM2.5 ölçümü: burada düzeltilecek bir şey yok.
report-event = Olay
report-no-event = Yok: kokusuz bir kontrol
report-observed = Gözlem
report-recorded = Kayıt
report-by = Kaydeden
report-none = yok
report-whole-home = tüm ev
report-edited = düzenlendi
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
settings-room-area = Alan
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
people-error-date-pick = Bu tarih için “bu günün sonunda” seçeneğini işaretleyin ya da tarihi silin.
people-error-past = Bitiş gelecekte olmalı.
people-error-not-expired = Bu hesabın süresi dolmadı.
people-error-resident = Ev sakinlerinin süresi hiç dolmaz.
people-error-username = Kullanıcı adı 3 ile 32 karakter olmalı: harf, rakam, ".", "_" veya "-".
people-error-username-taken = Bu kullanıcı adı alınmış.
people-invite = Birini davet et
people-new-setup-link = Yeni kurulum QR kodu
people-reset-password = Şifreyi sıfırla
invite-title = Birini davet et
invite-username-hint = Bu adla giriş yapacak. Harf, rakam, ".", "_" veya "-".
invite-role = Rol
invite-submit = QR kodunu oluştur
link-title = QR kodu
link-setup-title = { $username } için kurulum bağlantısı
link-reset-title = { $username } için şifre sıfırlama
link-scan = Bu kodu telefonunun kamerasıyla taratsın.
link-valid-until = Yalnızca bir kez, saat { $time } kadar geçerli (5 dakika).
link-text = Ya da bağlantıyı gönderin:
link-done = Tamam
accept-title = Yeni bir şifre seçin
accept-welcome = Hoş geldiniz, { $username }
accept-setup-intro = Hesabınızı tamamlamak için şifrenizi ve dilinizi seçin.
accept-reset-intro = { $username } için yeni bir şifre seçin. Bu hesaba bağlı tüm cihazlarda oturum kapatılır.
accept-password-hint = 10 ile 128 karakter arası.
accept-submit = Kaydet ve giriş yap
expiry-legend = Erişimin bitişi
expiry-1-day = 1 gün sonra
expiry-weekend = hafta sonundan sonra
expiry-1-week = 1 hafta sonra
expiry-2-weeks = 2 hafta sonra
expiry-date = bu günün sonunda:
expiry-never = hiçbir zaman

install-app = Uygulamayı yükle
install-card-hint = Her uygulama gibi ana ekrandan açın.
install-title = Smoke Tracker'ı ana ekranınıza ekleyin
install-welcome = Hesabınız hazır.
install-intro = Böylece doğrudan Kaydet ekranında bir uygulama gibi açılır ve kayıt birkaç saniye sürer.
install-button = Yükle
install-ios-title = iPhone ve iPad (Safari)
install-ios-share = Paylaş düğmesine dokunun
install-ios-add = "Ana Ekrana Ekle"yi seçin
install-ios-confirm = "Ekle"ye dokunun.
install-other = Android'de veya bilgisayarda Yükle düğmesi görünmezse tarayıcı menüsünü açıp "Uygulamayı yükle" veya "Ana ekrana ekle"yi seçin.
install-continue = Uygulamaya devam et
offline-title = Bağlantı yok
offline-text = Smoke Tracker'ın kaydettiklerinizi saklamak için internete ihtiyacı var. Yeniden çevrimiçi olduğunuzda tekrar deneyin.
offline-retry = Tekrar dene

graphs-title = Grafikler
graphs-filters = Filtreler
graphs-dates = Tarihler
graphs-range-month = Bu ay
graphs-range-7d = Son 7 gün
graphs-range-30d = Son 30 gün
graphs-range-year = Bu yıl
graphs-from = Başlangıç
graphs-to = Bitiş
graphs-where = Nerede
graphs-area-all = Tümü
graphs-by = Bildiren
graphs-everyone = Herkes
graphs-time = Günün saati
graphs-time-any = Herhangi
graphs-time-morning = Sabah
graphs-time-afternoon = Öğleden sonra
graphs-time-evening = Akşam
graphs-time-night = Gece
graphs-min-level = En düşük seviye
graphs-level-any = herhangi
graphs-level-from = seviye { $level }+
graphs-hide-clean = Temiz kontrolleri gizle
graphs-no-clean = temiz kontroller yok
graphs-reset = Sıfırla
graphs-show = Grafikleri göster
graphs-own-only = Bu grafikler yalnızca kendi bildirimlerinizi ve ölçümlerinizi gösterir.
graphs-none = Bu dönemde hiçbir kayıt yok.
graphs-needs-js = Bu grafik için JavaScript gerekir.
graphs-intensity = Zaman içinde koku yoğunluğu
graphs-pm25 = PM2.5 (µg/m³)
graphs-series-both = İkisi
graphs-series-intensity = Yoğunluk
graphs-series-pm25 = PM2.5
graphs-series-both-title = Koku yoğunluğu ve PM2.5
graphs-series-pm25-title = Zaman içinde PM2.5
graphs-lines-person = Kişiye göre
graphs-search-people = Kişi ara
graphs-events-day = Günlük olay sayısı
graphs-events-week = Haftalık olay sayısı
graphs-events-label = Olaylar
graphs-rooms = Günün saatlerine göre odalar
graphs-rooms-hint = Her odaya her saatte verilen ortalama seviye; 0 kontrol edildi ve temiz demektir. Boş bir hücre, o saatte o odayı kimsenin kontrol etmediği anlamına gelir.
graphs-rooms-none = Bu dönemde hiçbir oda kontrol edilmedi.
graphs-room = Oda
graphs-symptoms-label = Bildirimler
graphs-symptoms-none = Bu dönemde hiçbir belirti bildirilmedi.
graphs-areas = Ev içi ve ortak alanlar karşılaştırması
graphs-areas-hint = Her çubuk, o gün veya hafta bildirilen en yüksek seviyedir.

error-not-found-title = Sayfa bulunamadı
error-not-found = Bu sayfa mevcut değil.
error-forbidden-title = İstek engellendi
error-forbidden = Bu istek bu uygulamadan gelmediği için engellendi. Geri dönüp tekrar deneyin.
error-not-allowed-title = İzin yok
error-not-allowed = Hesabın bunu yapamaz.
error-internal-title = Bir şeyler ters gitti
error-internal = Bir şeyler ters gitti (ref: { $ref }).
error-link-title = Bu bağlantı artık çalışmıyor
error-link = Bağlantılar yalnızca bir kez ve 5 dakika geçerlidir. Bir ev sakininden yeni bir QR kodu isteyin.
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
