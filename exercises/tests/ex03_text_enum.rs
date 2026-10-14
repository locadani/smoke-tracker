use smoke_exercises::ex03_text_enum::UnknownValue;
use smoke_exercises::text_enum;

text_enum!(
    /// A made-up enum for the test.
    Colour {
        Red => "red",
        DarkGreen => "dark_green",
        Blue => "blue",
    }
);

text_enum!(Answer { Yes => "yes", No => "no" });

#[test]
fn each_variant_has_its_text() {
    assert_eq!(Colour::Red.as_str(), "red");
    assert_eq!(Colour::DarkGreen.as_str(), "dark_green");
    assert_eq!(Colour::Blue.as_str(), "blue");
}

#[test]
fn all_lists_the_variants_in_declaration_order() {
    assert_eq!(Colour::ALL, &[Colour::Red, Colour::DarkGreen, Colour::Blue]);
    assert_eq!(Answer::ALL, &[Answer::Yes, Answer::No]);
}

#[test]
fn text_parses_back_to_the_variant() {
    for colour in [Colour::Red, Colour::DarkGreen, Colour::Blue] {
        assert_eq!(colour.as_str().parse::<Colour>(), Ok(colour));
    }
}

#[test]
fn unknown_text_names_the_enum_and_the_value() {
    assert_eq!(
        "purple".parse::<Colour>(),
        Err(UnknownValue {
            kind: "Colour",
            value: "purple".to_owned()
        })
    );
    assert_eq!(
        "Yes".parse::<Answer>(),
        Err(UnknownValue {
            kind: "Answer",
            value: "Yes".to_owned()
        })
    );
}

#[test]
fn display_shows_the_text() {
    assert_eq!(Colour::DarkGreen.to_string(), "dark_green");
    assert_eq!(format!("{}", Answer::No), "no");
}
