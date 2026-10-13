//! Data written before a migration, and what each backend must make of it. Backends load the
//! rows into a database migrated up to the version before, then run the remaining migrations.

use smoke_domain::repos::ReportRepo;
use smoke_domain::types::{Level, ReportId, RoomId};

/// Rows as the schema of migration 0002 stores them: two rooms; report 1 at intensity 6 in
/// both, report 2 at intensity 4 in the second.
pub const ROWS_BEFORE_0003: &str = "\
    INSERT INTO users (id, username, role, locale, created_at) \
        VALUES (1, 'daniele', 'resident', 'en', 0); \
    INSERT INTO rooms (id, name, area) VALUES (1, 'Kitchen', 'inside'), (2, 'Bedroom', 'inside'); \
    INSERT INTO events (id, owner_id, opened_at, recorded_at) VALUES (1, 1, 0, 0); \
    INSERT INTO smell_reports (id, event_id, user_id, observed_at, recorded_at, intensity) \
        VALUES (1, 1, 1, 0, 0, 6), (2, 1, 1, 60000000, 60000000, 4); \
    INSERT INTO smell_report_rooms (smell_report_id, room_id) VALUES (1, 1), (1, 2), (2, 2);";

/// A report without rooms, which migration 0003 refuses to convert.
pub const REPORT_WITHOUT_ROOMS: &str = "\
    INSERT INTO smell_reports (id, event_id, user_id, observed_at, recorded_at, intensity) \
        VALUES (3, 1, 1, 120000000, 120000000, 5);";

/// The name the refusal carries in its error message.
pub const NEEDS_A_ROOM: &str = "every_smell_report_needs_a_room";

/// After migration 0003, every room of [`ROWS_BEFORE_0003`] has its report's intensity.
pub async fn each_room_took_its_reports_intensity<S: ReportRepo>(store: &S) {
    let levels = |id| async move {
        let report = store.report(ReportId(id)).await.unwrap().unwrap();
        report
            .values
            .rooms()
            .iter()
            .map(|r| (r.room, r.level))
            .collect::<Vec<_>>()
    };
    let level = |value| Level::new(value).unwrap();
    assert_eq!(
        levels(1).await,
        [(RoomId(1), level(6)), (RoomId(2), level(6))]
    );
    assert_eq!(levels(2).await, [(RoomId(2), level(4))]);
}
