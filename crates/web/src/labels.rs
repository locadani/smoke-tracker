//! Names shown for stored values: starter rooms and symptoms through their translation key,
//! custom ones as typed, and the translation keys of enum values.

use smoke_domain::records::{ReportValues, Room, RoomLevel, Symptom};
use smoke_domain::types::CloseReason;
use smoke_domain::types::RoomId;

use crate::i18n::Tr;

pub(crate) fn room(tr: Tr, room: &Room) -> String {
    match &room.i18n_key {
        Some(key) => tr.t(key),
        None => room.name.to_string(),
    }
}

/// The name of the room with this id among `rooms`; empty if there is none.
pub(crate) fn room_by_id(tr: Tr, rooms: &[Room], id: RoomId) -> String {
    rooms
        .iter()
        .find(|r| r.id == id)
        .map(|r| room(tr, r))
        .unwrap_or_default()
}

/// A room of a report with its level: "Living room 6".
pub(crate) fn room_level(tr: Tr, rooms: &[Room], at: RoomLevel) -> String {
    format!("{} {}", room_by_id(tr, rooms, at.room), at.level.get())
}

/// A report's rooms with their levels, "Living room 6, Bedroom 0"; "whole home" for a no-smell
/// report that lists none.
pub(crate) fn report_rooms(tr: Tr, rooms: &[Room], values: &ReportValues) -> String {
    if values.rooms().is_empty() {
        return tr.t("report-whole-home");
    }
    values
        .rooms()
        .iter()
        .map(|&at| room_level(tr, rooms, at))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn symptom(tr: Tr, symptom: &Symptom) -> String {
    match &symptom.i18n_key {
        Some(key) => tr.t(key),
        None => symptom.name.to_string(),
    }
}

pub(crate) fn close_reason(reason: CloseReason) -> &'static str {
    match reason {
        CloseReason::Manual => "close-reason-manual",
        CloseReason::NewEvent => "close-reason-new-event",
        CloseReason::Auto0400 => "close-reason-auto",
        CloseReason::Backdated => "close-reason-backdated",
    }
}

/// Every key this module returns, for the translation tests.
#[cfg(test)]
pub(crate) fn keys() -> Vec<&'static str> {
    CloseReason::ALL.iter().copied().map(close_reason).collect()
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
