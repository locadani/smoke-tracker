use smoke_exercises::ex12_report_values::{ReportError, ReportValues, RoomLevel};

fn room(room: u32, level: u8) -> RoomLevel {
    RoomLevel { room, level }
}

#[test]
fn rooms_and_symptoms_come_back_sorted_and_unique() {
    let values = ReportValues::new(
        vec![room(3, 2), room(1, 6), room(2, 0)],
        vec![9, 4, 9, 1, 4],
        None,
    )
    .unwrap();
    assert_eq!(values.rooms(), [room(1, 6), room(2, 0), room(3, 2)]);
    assert_eq!(values.symptoms(), [1, 4, 9]);
}

#[test]
fn a_room_listed_twice_is_refused_even_with_two_levels() {
    assert_eq!(
        ReportValues::new(vec![room(5, 1), room(2, 3), room(5, 7)], vec![], None),
        Err(ReportError::RoomListedTwice(5))
    );
}

#[test]
fn a_level_above_ten_is_refused() {
    assert_eq!(
        ReportValues::new(vec![room(1, 11)], vec![], None),
        Err(ReportError::InvalidLevel(11))
    );
    assert!(ReportValues::new(vec![room(1, 10)], vec![], None).is_ok());
}

#[test]
fn levels_are_checked_before_duplicates() {
    assert_eq!(
        ReportValues::new(vec![room(1, 3), room(1, 4), room(2, 99)], vec![], None),
        Err(ReportError::InvalidLevel(99))
    );
}

#[test]
fn a_note_is_trimmed_and_a_blank_note_is_no_note() {
    let noted = ReportValues::new(vec![], vec![], Some("  smelled in the hall \n")).unwrap();
    assert_eq!(noted.note(), Some("smelled in the hall"));
    for blank in ["", "   ", "\n\t"] {
        let values = ReportValues::new(vec![], vec![], Some(blank)).unwrap();
        assert_eq!(values.note(), None);
    }
    assert_eq!(
        ReportValues::new(vec![], vec![], None).unwrap().note(),
        None
    );
}

#[test]
fn a_note_is_limited_to_2000_characters_not_bytes() {
    let long_ok = "é".repeat(2000);
    assert_eq!(
        ReportValues::new(vec![], vec![], Some(&long_ok))
            .unwrap()
            .note(),
        Some(long_ok.as_str())
    );
    let too_long = "é".repeat(2001);
    assert_eq!(
        ReportValues::new(vec![], vec![], Some(&too_long)),
        Err(ReportError::NoteTooLong(2001))
    );
    let padded = format!("  {}  ", "x".repeat(2000));
    assert!(ReportValues::new(vec![], vec![], Some(&padded)).is_ok());
}

#[test]
fn the_level_of_a_room_is_none_when_it_was_not_checked() {
    let values = ReportValues::new(vec![room(1, 6), room(2, 0)], vec![], None).unwrap();
    assert_eq!(values.level_in(1), Some(6));
    assert_eq!(values.level_in(2), Some(0));
    assert_eq!(values.level_in(3), None);
}

#[test]
fn the_intensity_is_the_highest_room_level() {
    let values = ReportValues::new(vec![room(1, 2), room(2, 8), room(3, 5)], vec![], None).unwrap();
    assert_eq!(values.intensity(), 8);
}

#[test]
fn no_rooms_means_intensity_zero_and_no_smell() {
    let values = ReportValues::new(vec![], vec![], None).unwrap();
    assert_eq!(values.intensity(), 0);
    assert!(values.is_no_smell());
}

#[test]
fn rooms_all_at_zero_are_no_smell_but_one_smelly_room_is_not() {
    let clean = ReportValues::new(vec![room(1, 0), room(2, 0)], vec![], None).unwrap();
    assert!(clean.is_no_smell());
    let smelly = ReportValues::new(vec![room(1, 0), room(2, 1)], vec![], None).unwrap();
    assert!(!smelly.is_no_smell());
}
