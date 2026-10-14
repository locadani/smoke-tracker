use smoke_exercises::ex13_form_pairs::Pairs;

fn form() -> Pairs {
    Pairs::of(&[
        ("note", " smelled in the hall "),
        ("room", "3"),
        ("room", " 1 "),
        ("level-3", "6"),
        ("level-1", "2"),
        ("pm25", "   "),
        ("symptom", "7"),
    ])
}

#[test]
fn get_returns_the_first_value_trimmed() {
    let doubled = Pairs::of(&[("note", " first "), ("note", "second")]);
    assert_eq!(doubled.get("note"), Some("first"));
    assert_eq!(form().get("note"), Some("smelled in the hall"));
}

#[test]
fn get_treats_a_blank_value_like_a_missing_one() {
    assert_eq!(form().get("pm25"), None);
    assert_eq!(form().get("missing"), None);
}

#[test]
fn ids_collects_every_value_in_order() {
    assert_eq!(form().ids("room"), Some(vec![3, 1]));
    assert_eq!(form().ids("symptom"), Some(vec![7]));
}

#[test]
fn ids_of_an_absent_name_is_an_empty_list() {
    assert_eq!(form().ids("missing"), Some(vec![]));
}

#[test]
fn ids_is_none_when_any_value_is_not_a_number() {
    let form = Pairs::of(&[("room", "3"), ("room", "three"), ("room", "5")]);
    assert_eq!(form.ids("room"), None);
    assert_eq!(Pairs::of(&[("room", "")]).ids("room"), None);
}

#[test]
fn names_with_prefix_lists_what_follows_the_prefix() {
    let form = form();
    let ids: Vec<&str> = form.names_with_prefix("level-").collect();
    assert_eq!(ids, ["3", "1"]);
    assert_eq!(form.names_with_prefix("nothing-").count(), 0);
}

#[test]
fn the_names_borrow_from_the_form_not_from_a_copy() {
    let form = form();
    let first = form.names_with_prefix("level-").next().unwrap();
    let again = form.names_with_prefix("level-").next().unwrap();
    assert_eq!(first.as_ptr(), again.as_ptr());
}

#[test]
fn without_drops_the_named_fields_and_keeps_order_and_values() {
    let rest = form().without(&["room", "level-3", "level-1"]);
    assert_eq!(
        rest,
        [
            ("note".to_owned(), " smelled in the hall ".to_owned()),
            ("pm25".to_owned(), "   ".to_owned()),
            ("symptom".to_owned(), "7".to_owned()),
        ]
    );
    assert_eq!(form().without(&[]).len(), 7);
}
