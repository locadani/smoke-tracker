use std::error::Error;

use smoke_exercises::ex07_store_errors::{
    Conflict, StoreError, corrupt, read_level, read_level_text, read_levels, unique_conflict,
};

#[test]
fn the_named_column_decides_the_conflict() {
    assert_eq!(
        unique_conflict("UNIQUE constraint failed: users.username"),
        Conflict::UsernameTaken
    );
    assert_eq!(
        unique_conflict("UNIQUE constraint failed: index 'events_one_open'"),
        Conflict::AnotherEventOpen
    );
    assert_eq!(
        unique_conflict("UNIQUE constraint failed: audit_log.prev_hash"),
        Conflict::ChainForked
    );
}

#[test]
fn an_unrecognised_unique_failure_is_a_duplicate() {
    assert_eq!(
        unique_conflict("UNIQUE constraint failed: invites.token_hash"),
        Conflict::Duplicate
    );
    assert_eq!(unique_conflict(""), Conflict::Duplicate);
}

#[test]
fn errors_print_a_short_message() {
    assert_eq!(
        StoreError::Conflict(Conflict::Duplicate).to_string(),
        "that write conflicts with stored data"
    );
    assert_eq!(
        corrupt("note is not trimmed").to_string(),
        "stored data is corrupt: note is not trimmed"
    );
    assert_eq!(
        StoreError::Backend("disk full".into()).to_string(),
        "database error"
    );
}

#[test]
fn only_a_backend_error_has_a_source() {
    let backend = StoreError::Backend("disk full".into());
    assert_eq!(backend.source().unwrap().to_string(), "disk full");
    assert!(StoreError::Conflict(Conflict::Duplicate).source().is_none());
    assert!(corrupt("x").source().is_none());
}

#[test]
fn a_conflict_converts_into_a_store_error() {
    let error: StoreError = Conflict::UsernameTaken.into();
    assert!(matches!(
        error,
        StoreError::Conflict(Conflict::UsernameTaken)
    ));
}

#[test]
fn corrupt_accepts_anything_that_displays() {
    assert!(matches!(corrupt(42), StoreError::Corrupt(reason) if reason == "42"));
    assert!(matches!(corrupt(String::from("a")), StoreError::Corrupt(reason) if reason == "a"));
}

#[test]
fn levels_from_0_to_10_are_read() {
    assert_eq!(read_level(0).unwrap(), 0);
    assert_eq!(read_level(10).unwrap(), 10);
}

#[test]
fn a_level_out_of_range_is_corrupt_data() {
    for stored in [11, -1, 300, i64::MAX] {
        let error = read_level(stored).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("stored data is corrupt: level {stored} is out of range")
        );
    }
}

#[test]
fn many_levels_stop_at_the_first_corrupt_one() {
    assert_eq!(read_levels(&[1, 2, 10]).unwrap(), [1, 2, 10]);
    assert_eq!(read_levels(&[]).unwrap(), Vec::<u8>::new());
    let error = read_levels(&[1, 99, -5]).unwrap_err();
    assert_eq!(
        error.to_string(),
        "stored data is corrupt: level 99 is out of range"
    );
}

#[test]
fn text_levels_parse_then_check() {
    assert_eq!(read_level_text("7").unwrap(), 7);
    assert_eq!(
        read_level_text("12").unwrap_err().to_string(),
        "stored data is corrupt: level 12 is out of range"
    );
    let error = read_level_text("seven").unwrap_err();
    assert!(error.to_string().starts_with("stored data is corrupt: "));
    assert!(matches!(error, StoreError::Corrupt(_)));
}
