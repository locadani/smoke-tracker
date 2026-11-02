//! The commands against a temporary SQLite database.

use std::sync::Arc;

use smoke_app::backend::Backend;
use smoke_app::commands::{create_resident, rename_user, verify_audit};
use smoke_domain::clock::SystemClock;
use smoke_domain::repos::{AuditRepo, Conflict, ListRepo, StoreError, UserRepo};
use smoke_domain::types::{Locale, NewPassword, Role, Username};
use smoke_storage_sqlite::SqliteStore;
use smoke_web::password;
use sqlx::SqlitePool;
use tempfile::TempDir;

const PASSWORD: &str = "correct horse battery";

async fn prepared() -> (TempDir, String, SqliteStore) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("app.db").display());
    let backend = Backend::connect(&url, Arc::new(SystemClock)).await.unwrap();
    backend.prepare().await.unwrap();
    let Backend::Sqlite(store) = backend else {
        unreachable!("a sqlite URL")
    };
    (dir, url, store)
}

fn username(name: &str) -> Username {
    Username::new(name).unwrap()
}

fn password() -> NewPassword {
    NewPassword::new(PASSWORD.to_owned()).unwrap()
}

#[tokio::test]
async fn prepare_can_run_again_and_seeds_the_lists_once() {
    let (_dir, url, store) = prepared().await;
    let again = Backend::connect(&url, Arc::new(SystemClock)).await.unwrap();
    again.prepare().await.unwrap();
    assert_eq!(store.rooms().await.unwrap().len(), 6);
    assert_eq!(store.symptoms().await.unwrap().len(), 6);
    assert_eq!(store.audit_log().await.unwrap().len(), 12);
}

#[tokio::test]
async fn an_unknown_scheme_is_refused_before_connecting() {
    let error = Backend::connect("mysql://db/smoke", Arc::new(SystemClock))
        .await
        .err()
        .unwrap();
    assert_eq!(
        error.to_string(),
        "DATABASE_URL must start with sqlite: or postgres://"
    );
}

#[tokio::test]
async fn create_resident_stores_an_argon2_hash_and_audits_without_it() {
    let (_dir, _url, store) = prepared().await;
    let user = create_resident::create(&store, username(" Daniele "), password(), Locale::It)
        .await
        .unwrap();
    assert_eq!(user.username.as_str(), "daniele");
    assert_eq!(
        (user.role, user.locale, user.expires_at),
        (Role::Resident, Locale::It, None)
    );
    let stored = store.user(user.id).await.unwrap().unwrap();
    assert!(password::verify(stored.password_hash.as_ref(), PASSWORD));
    let log = format!("{:?}", store.audit_log().await.unwrap());
    assert!(!log.contains("argon2"), "the hash reached the audit log");
}

#[tokio::test]
async fn create_resident_refuses_a_taken_username() {
    let (_dir, _url, store) = prepared().await;
    create_resident::create(&store, username("daniele"), password(), Locale::En)
        .await
        .unwrap();
    let error = create_resident::create(&store, username("DANIELE"), password(), Locale::En)
        .await
        .unwrap_err();
    assert!(
        matches!(
            error.downcast_ref::<StoreError>(),
            Some(StoreError::Conflict(Conflict::UsernameTaken))
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn rename_user_keeps_verify_audit_clean() {
    let (_dir, _url, store) = prepared().await;
    let user = create_resident::create(&store, username("dugyu"), password(), Locale::En)
        .await
        .unwrap();
    let renamed = rename_user::rename(&store, &username("dugyu"), username("Duygu"))
        .await
        .unwrap();
    assert_eq!((renamed.id, renamed.username.as_str()), (user.id, "duygu"));
    assert!(verify_audit::run(&store).await.unwrap().is_ok());
}

#[tokio::test]
async fn rename_user_refuses_an_unknown_or_unchanged_name() {
    let (_dir, _url, store) = prepared().await;
    create_resident::create(&store, username("daniele"), password(), Locale::En)
        .await
        .unwrap();
    let error = rename_user::rename(&store, &username("nobody"), username("someone"))
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "no user is called nobody");
    let error = rename_user::rename(&store, &username("daniele"), username("DANIELE"))
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "daniele already has that username");
}

#[tokio::test]
async fn rename_user_refuses_a_taken_username() {
    let (_dir, _url, store) = prepared().await;
    for name in ["daniele", "duygu"] {
        create_resident::create(&store, username(name), password(), Locale::En)
            .await
            .unwrap();
    }
    let error = rename_user::rename(&store, &username("duygu"), username("daniele"))
        .await
        .unwrap_err();
    assert!(
        matches!(
            error.downcast_ref::<StoreError>(),
            Some(StoreError::Conflict(Conflict::UsernameTaken))
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn verify_audit_passes_on_honest_data() {
    let (_dir, _url, store) = prepared().await;
    create_resident::create(&store, username("daniele"), password(), Locale::En)
        .await
        .unwrap();
    let report = verify_audit::run(&store).await.unwrap();
    assert!(report.is_ok());
    assert_eq!(report.entries, 13);
    let text = verify_audit::describe(&report);
    assert!(text.contains("hash chain:    ok"), "{text}");
    assert!(text.contains(&report.head_hash), "{text}");
}

#[tokio::test]
async fn verify_audit_reports_a_direct_database_edit() {
    let (_dir, url, store) = prepared().await;
    let pool = SqlitePool::connect(&url).await.unwrap();
    sqlx::raw_sql("UPDATE rooms SET name = 'Garage' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let report = verify_audit::run(&store).await.unwrap();
    assert!(!report.is_ok());
    let text = verify_audit::describe(&report);
    assert!(
        text.contains("room 1: differs from audit entry 1 (edited outside the app)"),
        "{text}"
    );
}

mod photo_files {
    use std::path::Path;

    use chrono::{DateTime, Utc};
    use smoke_app::backend::BackendKind;
    use smoke_app::commands::verify_audit::{PhotoCheck, PhotoProblem, check_photos, photo_store};
    use smoke_app::config::PhotoConfig;
    use smoke_domain::records::{NewEntry, NewReading, Pm25Photo};
    use smoke_domain::repos::{Commit, EntryWrites, Transactional};
    use smoke_domain::types::Pm25;
    use smoke_photos::{LocalPhotoStore, PhotoStore, validate};

    use super::*;

    fn at() -> DateTime<Utc> {
        "2026-11-02T13:35:00Z".parse().unwrap()
    }

    async fn stored_photo(photos: &PhotoStore, body: u8) -> Pm25Photo {
        let mut bytes = vec![0xFF, 0xD8, 0xFF, body];
        bytes.extend_from_slice(&[body; 40]);
        let valid = validate(bytes, 10_000).unwrap();
        let len = u32::try_from(valid.bytes.len()).unwrap();
        photos.put_pending(&valid).await.unwrap();
        let key = photos.promote(&valid.sha256, len, at()).await.unwrap();
        Pm25Photo {
            key,
            bytes: len,
            sha256: valid.sha256,
            taken_at: at(),
        }
    }

    async fn entry_with(store: &SqliteStore, photo: Pm25Photo) -> i64 {
        let user = create_resident::create(store, username("daniele"), password(), Locale::En)
            .await
            .unwrap();
        let new = NewEntry::new(
            user.id,
            None,
            at(),
            None,
            None,
            Some(NewReading {
                value: Pm25::new(12.0).unwrap(),
                photo: Some(photo),
            }),
        )
        .unwrap();
        let mut tx = store.begin(Some(user.id)).await.unwrap();
        let entry = tx.create_entry(new).await.unwrap();
        tx.commit().await.unwrap();
        entry.id.0
    }

    fn local(dir: &Path) -> PhotoStore {
        PhotoStore::Local(LocalPhotoStore::new(dir.to_path_buf()))
    }

    #[tokio::test]
    async fn verify_audit_passes_with_intact_photo_files() {
        let (_dir, _url, store) = prepared().await;
        let root = tempfile::tempdir().unwrap();
        let photos = local(root.path());
        let photo = stored_photo(&photos, 1).await;
        entry_with(&store, photo).await;
        let check = check_photos(&store, &photos).await.unwrap();
        assert_eq!(check, PhotoCheck::Checked(vec![]));
        assert_eq!(check.describe(), "photo files:   ok\n");
    }

    #[tokio::test]
    async fn verify_audit_reports_a_changed_photo_file() {
        let (_dir, _url, store) = prepared().await;
        let root = tempfile::tempdir().unwrap();
        let photos = local(root.path());
        let photo = stored_photo(&photos, 1).await;
        std::fs::write(root.path().join(photo.key.as_str()), b"\xFF\xD8\xFFedited").unwrap();
        let id = entry_with(&store, photo).await;
        let check = check_photos(&store, &photos).await.unwrap();
        assert!(!check.is_ok());
        assert!(
            check
                .describe()
                .contains(&format!("photo file changed or missing: entry {id}")),
            "{}",
            check.describe()
        );
    }

    #[tokio::test]
    async fn verify_audit_reports_a_missing_photo_file() {
        let (_dir, _url, store) = prepared().await;
        let root = tempfile::tempdir().unwrap();
        let photos = local(root.path());
        let photo = stored_photo(&photos, 1).await;
        std::fs::remove_file(root.path().join(photo.key.as_str())).unwrap();
        let id = entry_with(&store, photo).await;
        let check = check_photos(&store, &photos).await.unwrap();
        assert_eq!(
            check,
            PhotoCheck::Checked(vec![PhotoProblem::ChangedOrMissing { entry_id: id }])
        );
    }

    #[tokio::test]
    async fn verify_audit_reports_a_photo_without_entry() {
        let (_dir, _url, store) = prepared().await;
        let root = tempfile::tempdir().unwrap();
        let photos = local(root.path());
        let kept = stored_photo(&photos, 1).await;
        let stray = stored_photo(&photos, 2).await;
        entry_with(&store, kept).await;
        let check = check_photos(&store, &photos).await.unwrap();
        assert!(
            check
                .describe()
                .contains(&format!("photo file without entry: {}", stray.key.as_str())),
            "{}",
            check.describe()
        );
    }

    #[tokio::test]
    async fn verify_audit_counts_a_deleted_entry_as_referencing_its_photo() {
        let (_dir, _url, store) = prepared().await;
        let root = tempfile::tempdir().unwrap();
        let photos = local(root.path());
        let photo = stored_photo(&photos, 1).await;
        let id = entry_with(&store, photo).await;
        let user = store
            .user_by_username(&username("daniele"))
            .await
            .unwrap()
            .unwrap();
        let mut tx = store.begin(Some(user.id)).await.unwrap();
        tx.delete_entry(smoke_domain::types::LogEntryId(id))
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(
            check_photos(&store, &photos).await.unwrap(),
            PhotoCheck::Checked(vec![])
        );
    }

    #[tokio::test]
    async fn verify_audit_without_a_store_says_not_checked() {
        let (_dir, _url, store) = prepared().await;
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("not-a-folder");
        std::fs::write(&file, b"x").unwrap();
        let check = check_photos(&store, &local(&file)).await.unwrap();
        assert!(check.is_ok());
        assert!(
            check.describe().starts_with("photo files not checked"),
            "{}",
            check.describe()
        );
    }

    #[test]
    fn the_production_database_without_a_bucket_is_not_checked() {
        let lookup = |pairs: &'static [(&str, &str)]| {
            PhotoConfig::from_lookup(move |name| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == name)
                    .map(|(_, v)| (*v).to_owned())
            })
            .unwrap()
        };
        let none = lookup(&[]);
        let reason = photo_store(BackendKind::Postgres, &none).unwrap_err();
        assert_eq!(reason, "PHOTO_BUCKET is not set");
        assert!(photo_store(BackendKind::Sqlite, &none).is_ok());
    }
}
