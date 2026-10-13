//! Core value types. Constructors validate, so an invalid value can never exist.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::DomainError;

macro_rules! id_type {
    ($($(#[$meta:meta])* $name:ident),+ $(,)?) => {$(
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub i64);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    )+};
}

id_type!(
    /// Identifies a user account.
    UserId,
    /// Identifies a smoke event (one incident).
    EventId,
    /// Identifies a smell report.
    ReportId,
    /// Identifies a PM2.5 reading.
    ReadingId,
    /// Identifies a room.
    RoomId,
    /// Identifies a symptom.
    SymptomId,
);

/// Defines a field-less enum stored as text: `as_str`, `FromStr`, `Display`, serde, and `ALL`.
macro_rules! str_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $($(#[$vmeta:meta])* $variant:ident => $text:literal),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vmeta])* $variant),+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// The stable text form, used in the database and in serde.
            pub fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $text),+
                }
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = $crate::DomainError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok($name::$variant),)+
                    _ => Err($crate::DomainError::UnknownValue {
                        kind: stringify!($name),
                        value: s.to_owned(),
                    }),
                }
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = <String as ::serde::Deserialize>::deserialize(deserializer)?;
                text.parse().map_err(::serde::de::Error::custom)
            }
        }
    };
}
pub(crate) use str_enum;

str_enum!(
    /// What a user is allowed to do; see `permissions`.
    Role {
        Resident => "resident",
        Guest => "guest",
        Viewer => "viewer",
    }
);

str_enum!(
    /// Whether a room is inside the apartment or a shared common area.
    RoomArea {
        Inside => "inside",
        Common => "common",
    }
);

str_enum!(
    /// Why an event was closed.
    CloseReason {
        /// A user closed it.
        Manual => "manual",
        /// Someone started a new event while this one was open.
        NewEvent => "new_event",
        /// The nightly 04:00 sweep closed it.
        Auto0400 => "auto_0400",
        /// It was created in the past, already over.
        Backdated => "backdated",
    }
);

str_enum!(
    /// Interface language.
    Locale {
        En => "en",
        It => "it",
        Tr => "tr",
    }
);

/// A login name. Construction trims and lowercases it, so "Daniele " and "daniele" are the same
/// account.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Username(String);

impl Username {
    pub const MIN_LEN: usize = 3;
    pub const MAX_LEN: usize = 32;

    pub fn new(raw: &str) -> Result<Self, DomainError> {
        let name = raw.trim().to_ascii_lowercase();
        let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || "._-".contains(c);
        if (Self::MIN_LEN..=Self::MAX_LEN).contains(&name.len()) && name.chars().all(allowed) {
            Ok(Self(name))
        } else {
            Err(DomainError::InvalidUsername(raw.to_owned()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The name of a room or symptom, as typed by a resident.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ListName(String);

impl ListName {
    pub const MAX_CHARS: usize = 60;

    pub fn new(raw: &str) -> Result<Self, DomainError> {
        let name = raw.trim();
        if name.is_empty() || name.chars().count() > Self::MAX_CHARS {
            Err(DomainError::InvalidListName(raw.to_owned()))
        } else {
            Ok(Self(name.to_owned()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ListName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A password someone is choosing: 10–128 characters, no composition rules (spec §8.1). Kept
/// exactly as typed, never trimmed, and never printed.
#[derive(Clone, PartialEq, Eq)]
pub struct NewPassword(String);

impl NewPassword {
    pub const MIN_CHARS: usize = 10;
    pub const MAX_CHARS: usize = 128;

    pub fn new(raw: String) -> Result<Self, DomainError> {
        let chars = raw.chars().count();
        if (Self::MIN_CHARS..=Self::MAX_CHARS).contains(&chars) {
            Ok(Self(raw))
        } else {
            Err(DomainError::InvalidPasswordLength(chars))
        }
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for NewPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("NewPassword(<redacted>)")
    }
}

/// How strong a smell is, on the personal 1–10 scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct Intensity(u8);

impl Intensity {
    pub const MIN: u8 = 1;
    pub const MAX: u8 = 10;

    pub fn new(value: u8) -> Result<Self, DomainError> {
        if (Self::MIN..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(DomainError::InvalidIntensity(value))
        }
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Intensity {
    type Error = DomainError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Intensity> for u8 {
    fn from(intensity: Intensity) -> Self {
        intensity.0
    }
}

/// A PM2.5 concentration in µg/m³.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct Pm25(f64);

impl Pm25 {
    pub const MAX: f64 = 1000.0;

    pub fn new(ugm3: f64) -> Result<Self, DomainError> {
        if ugm3.is_finite() && (0.0..=Self::MAX).contains(&ugm3) {
            Ok(Self(ugm3))
        } else {
            Err(DomainError::InvalidPm25(ugm3))
        }
    }

    pub fn ugm3(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Pm25 {
    type Error = DomainError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Pm25> for f64 {
    fn from(pm25: Pm25) -> Self {
        pm25.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intensity_accepts_the_full_range() {
        for value in 1..=10 {
            assert_eq!(Intensity::new(value).unwrap().get(), value);
        }
    }

    #[test]
    fn intensity_rejects_values_outside_one_to_ten() {
        assert_eq!(Intensity::new(0), Err(DomainError::InvalidIntensity(0)));
        assert_eq!(Intensity::new(11), Err(DomainError::InvalidIntensity(11)));
    }

    #[test]
    fn intensity_cannot_be_deserialised_out_of_range() {
        assert!(serde_json::from_str::<Intensity>("11").is_err());
        assert_eq!(serde_json::from_str::<Intensity>("7").unwrap().get(), 7);
        assert_eq!(
            serde_json::to_string(&Intensity::new(7).unwrap()).unwrap(),
            "7"
        );
    }

    #[test]
    fn pm25_accepts_zero_to_one_thousand() {
        assert_eq!(Pm25::new(0.0).unwrap().ugm3(), 0.0);
        assert_eq!(Pm25::new(42.5).unwrap().ugm3(), 42.5);
        assert_eq!(Pm25::new(1000.0).unwrap().ugm3(), 1000.0);
    }

    #[test]
    fn pm25_rejects_negative_too_large_and_non_finite() {
        for bad in [-0.1, 1000.1, f64::NAN, f64::INFINITY] {
            assert!(Pm25::new(bad).is_err(), "{bad} should be rejected");
        }
        assert!(serde_json::from_str::<Pm25>("-5").is_err());
    }

    #[test]
    fn string_enums_round_trip_through_text_and_serde() {
        for role in Role::ALL {
            assert_eq!(role.as_str().parse::<Role>().unwrap(), *role);
            let json = serde_json::to_string(role).unwrap();
            assert_eq!(serde_json::from_str::<Role>(&json).unwrap(), *role);
        }
        for reason in CloseReason::ALL {
            assert_eq!(reason.as_str().parse::<CloseReason>().unwrap(), *reason);
        }
        for area in RoomArea::ALL {
            assert_eq!(area.as_str().parse::<RoomArea>().unwrap(), *area);
        }
        for locale in Locale::ALL {
            assert_eq!(locale.as_str().parse::<Locale>().unwrap(), *locale);
        }
    }

    #[test]
    fn string_enums_use_the_stored_spelling() {
        assert_eq!(CloseReason::Auto0400.as_str(), "auto_0400");
        assert_eq!(
            serde_json::to_string(&CloseReason::NewEvent).unwrap(),
            "\"new_event\""
        );
        assert_eq!(Role::Guest.to_string(), "guest");
    }

    #[test]
    fn unknown_strings_are_rejected_with_the_type_name() {
        assert_eq!(
            "admin".parse::<Role>(),
            Err(DomainError::UnknownValue {
                kind: "Role",
                value: "admin".into()
            })
        );
        assert!(serde_json::from_str::<Role>("\"admin\"").is_err());
    }

    #[test]
    fn ids_serialise_as_plain_numbers() {
        assert_eq!(serde_json::to_string(&UserId(7)).unwrap(), "7");
        assert_eq!(EventId(3).to_string(), "3");
    }

    #[test]
    fn usernames_are_normalised() {
        assert_eq!(Username::new("  Daniele ").unwrap().as_str(), "daniele");
        assert_eq!(
            Username::new("Daniele").unwrap(),
            Username::new("daniele").unwrap()
        );
        assert_eq!(
            Username::new("guest.2026_a-b").unwrap().to_string(),
            "guest.2026_a-b"
        );
    }

    #[test]
    fn usernames_reject_bad_lengths_and_characters() {
        for bad in [
            "ab",
            "",
            "   ",
            "has space",
            "ünïcode",
            "semi;colon",
            &"x".repeat(33),
        ] {
            assert_eq!(
                Username::new(bad),
                Err(DomainError::InvalidUsername(bad.to_owned())),
                "{bad:?}"
            );
        }
        assert!(Username::new(&"x".repeat(32)).is_ok());
        assert!(Username::new("abc").is_ok());
    }

    #[test]
    fn list_names_are_trimmed_and_bounded() {
        assert_eq!(ListName::new("  Balcony ").unwrap().as_str(), "Balcony");
        assert_eq!(
            ListName::new("Çalışma odası").unwrap().as_str(),
            "Çalışma odası"
        );
        assert!(ListName::new(&"ç".repeat(60)).is_ok());
        for bad in ["", "   ", &"x".repeat(61)] {
            assert_eq!(
                ListName::new(bad),
                Err(DomainError::InvalidListName(bad.to_owned()))
            );
        }
    }

    #[test]
    fn passwords_need_ten_to_128_characters_counted_as_characters() {
        assert!(NewPassword::new("x".repeat(10)).is_ok());
        assert!(NewPassword::new("ş".repeat(128)).is_ok());
        assert_eq!(
            NewPassword::new("x".repeat(9)),
            Err(DomainError::InvalidPasswordLength(9))
        );
        assert_eq!(
            NewPassword::new("x".repeat(129)),
            Err(DomainError::InvalidPasswordLength(129))
        );
    }

    #[test]
    fn passwords_are_kept_as_typed_and_never_printed() {
        let password = NewPassword::new("  spaced out  ".to_owned()).unwrap();
        assert_eq!(password.expose(), "  spaced out  ");
        assert_eq!(format!("{password:?}"), "NewPassword(<redacted>)");
    }
}
