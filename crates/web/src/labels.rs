//! Names shown for stored values: starter rooms and symptoms through their translation key,
//! custom ones as typed.

use smoke_domain::records::{Room, Symptom};

use crate::i18n::Tr;

pub(crate) fn room(tr: Tr, room: &Room) -> String {
    match &room.i18n_key {
        Some(key) => tr.t(key),
        None => room.name.to_string(),
    }
}

pub(crate) fn symptom(tr: Tr, symptom: &Symptom) -> String {
    match &symptom.i18n_key {
        Some(key) => tr.t(key),
        None => symptom.name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use smoke_domain::types::{ListName, Locale, RoomArea, RoomId, SymptomId};

    use super::*;

    #[test]
    fn starter_items_are_translated_and_custom_ones_shown_as_typed() {
        let starter = Room {
            id: RoomId(3),
            name: ListName::new("Bedroom").unwrap(),
            i18n_key: Some("room-bedroom".to_owned()),
            area: RoomArea::Inside,
            hidden_at: None,
        };
        let custom = Room {
            i18n_key: None,
            name: ListName::new("Balcone").unwrap(),
            ..starter.clone()
        };
        assert_eq!(room(Tr(Locale::It), &starter), "Camera da letto");
        assert_eq!(room(Tr(Locale::Tr), &custom), "Balcone");
        let cough = Symptom {
            id: SymptomId(1),
            name: ListName::new("Cough").unwrap(),
            i18n_key: Some("symptom-cough".to_owned()),
            hidden_at: None,
        };
        assert_eq!(symptom(Tr(Locale::Tr), &cough), "Öksürük");
    }
}
