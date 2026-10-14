use smoke_exercises::ex01_newtypes::{Level, Password, RuleError, Username};

#[test]
fn levels_from_0_to_10_are_valid() {
    for value in 0..=10 {
        assert_eq!(Level::new(value).unwrap().get(), value);
    }
}

#[test]
fn a_level_above_10_is_refused_with_the_value() {
    assert_eq!(Level::new(11), Err(RuleError::InvalidLevel(11)));
    assert_eq!(Level::new(255), Err(RuleError::InvalidLevel(255)));
}

#[test]
fn only_zero_is_clean() {
    assert!(Level::CLEAN.is_clean());
    assert!(Level::new(0).unwrap().is_clean());
    assert!(!Level::new(1).unwrap().is_clean());
}

#[test]
fn levels_compare_by_value() {
    assert!(Level::new(3).unwrap() < Level::new(7).unwrap());
    let highest = [2, 9, 4]
        .into_iter()
        .map(|v| Level::new(v).unwrap())
        .max()
        .unwrap();
    assert_eq!(highest.get(), 9);
}

#[test]
fn levels_convert_both_ways() {
    assert_eq!(Level::try_from(5).unwrap().get(), 5);
    assert_eq!(Level::try_from(12), Err(RuleError::InvalidLevel(12)));
    assert_eq!(u8::from(Level::new(8).unwrap()), 8);
}

#[test]
fn a_username_is_trimmed_and_lowercased() {
    let name = Username::new("  Daniele ").unwrap();
    assert_eq!(name.as_str(), "daniele");
    assert_eq!(name.to_string(), "daniele");
}

#[test]
fn usernames_allow_letters_digits_dot_underscore_and_dash() {
    assert!(Username::new("a.b_c-d9").is_ok());
}

#[test]
fn usernames_have_3_to_32_characters() {
    assert!(Username::new("abc").is_ok());
    assert!(Username::new(&"a".repeat(32)).is_ok());
    assert_eq!(
        Username::new("ab"),
        Err(RuleError::InvalidUsername("ab".to_owned()))
    );
    assert!(Username::new(&"a".repeat(33)).is_err());
}

#[test]
fn other_characters_are_refused_and_the_error_keeps_what_was_typed() {
    assert_eq!(
        Username::new(" A B "),
        Err(RuleError::InvalidUsername(" A B ".to_owned()))
    );
    assert!(Username::new("café").is_err());
    assert!(Username::new("a/b").is_err());
}

#[test]
fn a_password_needs_10_to_128_characters() {
    assert_eq!(
        Password::new("123456789".to_owned()),
        Err(RuleError::InvalidPasswordLength(9))
    );
    assert!(Password::new("1234567890".to_owned()).is_ok());
    assert!(Password::new("x".repeat(128)).is_ok());
    assert_eq!(
        Password::new("x".repeat(129)),
        Err(RuleError::InvalidPasswordLength(129))
    );
}

#[test]
fn a_password_counts_characters_not_bytes() {
    assert!(Password::new("é".repeat(10)).is_ok());
    assert_eq!(
        Password::new("é".repeat(5)),
        Err(RuleError::InvalidPasswordLength(5))
    );
}

#[test]
fn a_password_is_kept_exactly_as_typed_and_never_printed() {
    let password = Password::new("  spaces and Capitals  ".to_owned()).unwrap();
    assert_eq!(password.expose(), "  spaces and Capitals  ");
    assert_eq!(format!("{password:?}"), "Password(<redacted>)");
}
