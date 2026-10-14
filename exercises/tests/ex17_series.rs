use smoke_exercises::ex17_series::{
    Area, AreaLevels, LevelTally, PersonLine, Report, area_levels, intensity_lines, room_hours,
    symptom_counts,
};

fn report(user: u32, at: u64, hour: usize, rooms: &[(u32, u8)], symptoms: &[u32]) -> Report {
    Report {
        user,
        at,
        hour,
        rooms: rooms.to_vec(),
        symptoms: symptoms.to_vec(),
    }
}

#[test]
fn each_person_gets_a_line_in_time_order_with_the_highest_room_level() {
    let reports = [
        report(2, 300, 9, &[(1, 4)], &[]),
        report(1, 200, 8, &[(1, 2), (2, 7)], &[]),
        report(1, 100, 7, &[(3, 5)], &[]),
        report(2, 50, 6, &[], &[]),
    ];
    assert_eq!(
        intensity_lines(&reports),
        [
            PersonLine {
                user: 1,
                points: vec![(100, 5), (200, 7)]
            },
            PersonLine {
                user: 2,
                points: vec![(50, 0), (300, 4)]
            },
        ]
    );
}

#[test]
fn no_reports_no_lines() {
    assert!(intensity_lines(&[]).is_empty());
}

#[test]
fn a_tally_averages_and_has_no_mean_without_checks() {
    assert_eq!(LevelTally::default().mean(), None);
    assert_eq!(
        LevelTally {
            checks: 4,
            total: 6
        }
        .mean(),
        Some(1.5)
    );
    assert_eq!(
        LevelTally {
            checks: 1,
            total: 0
        }
        .mean(),
        Some(0.0)
    );
}

#[test]
fn the_heatmap_averages_levels_per_room_and_hour_counting_clean_checks() {
    let reports = [
        report(1, 1, 22, &[(1, 6), (2, 0)], &[]),
        report(1, 2, 22, &[(1, 2)], &[]),
        report(1, 3, 23, &[(2, 4)], &[]),
        report(1, 4, 5, &[], &[]),
    ];
    let rows = room_hours(&reports);
    assert_eq!(rows.len(), 2);
    assert_eq!((rows[0].room, rows[1].room), (1, 2));
    assert_eq!(
        rows[0].hours[22],
        LevelTally {
            checks: 2,
            total: 8
        }
    );
    assert_eq!(rows[0].hours[22].mean(), Some(4.0));
    assert_eq!(rows[0].hours[23], LevelTally::default());
    assert_eq!(
        rows[1].hours[22],
        LevelTally {
            checks: 1,
            total: 0
        }
    );
    assert_eq!(rows[1].hours[23].mean(), Some(4.0));
    assert_eq!(rows[1].hours.len(), 24);
}

#[test]
fn rooms_nobody_named_are_left_out() {
    assert!(room_hours(&[report(1, 1, 3, &[], &[])]).is_empty());
    assert!(room_hours(&[]).is_empty());
}

#[test]
fn symptoms_are_counted_most_frequent_first_then_by_id() {
    let reports = [
        report(1, 1, 1, &[], &[3, 1]),
        report(1, 2, 1, &[], &[3]),
        report(1, 3, 1, &[], &[2, 3]),
        report(1, 4, 1, &[], &[2]),
        report(1, 5, 1, &[], &[]),
    ];
    assert_eq!(symptom_counts(&reports), [(3, 3), (2, 2), (1, 1)]);
}

#[test]
fn equal_counts_fall_back_to_the_lower_id() {
    let reports = [report(1, 1, 1, &[], &[9]), report(1, 2, 1, &[], &[4])];
    assert_eq!(symptom_counts(&reports), [(4, 1), (9, 1)]);
}

#[test]
fn each_area_keeps_its_highest_level() {
    let areas = [(1, Area::Inside), (2, Area::Inside), (3, Area::Common)];
    let reports = [
        report(1, 1, 1, &[(1, 3), (3, 8)], &[]),
        report(1, 2, 1, &[(2, 6)], &[]),
        report(1, 3, 1, &[(3, 2), (99, 10)], &[]),
    ];
    assert_eq!(
        area_levels(&reports, &areas),
        AreaLevels {
            inside: Some(6),
            common: Some(8)
        }
    );
}

#[test]
fn an_area_nobody_checked_is_none_and_a_clean_check_is_zero_not_none() {
    let areas = [(1, Area::Inside), (3, Area::Common)];
    let only_inside = [report(1, 1, 1, &[(1, 0)], &[])];
    assert_eq!(
        area_levels(&only_inside, &areas),
        AreaLevels {
            inside: Some(0),
            common: None
        }
    );
    assert_eq!(area_levels(&[], &areas), AreaLevels::default());
}
