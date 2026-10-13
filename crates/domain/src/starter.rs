//! The rooms and symptoms a new installation starts with (spec §6). They are shown through
//! their translation keys; the English name is stored as the fallback.

use crate::records::{NewRoom, NewSymptom};
use crate::repos::{Commit, ListRepo, ListWrites, StoreResult, Transactional};
use crate::types::{ListName, RoomArea};

pub struct StarterRoom {
    pub i18n_key: &'static str,
    pub name: &'static str,
    pub area: RoomArea,
}

pub struct StarterSymptom {
    pub i18n_key: &'static str,
    pub name: &'static str,
}

const fn room(i18n_key: &'static str, name: &'static str, area: RoomArea) -> StarterRoom {
    StarterRoom {
        i18n_key,
        name,
        area,
    }
}

const fn symptom(i18n_key: &'static str, name: &'static str) -> StarterSymptom {
    StarterSymptom { i18n_key, name }
}

pub const STARTER_ROOMS: [StarterRoom; 6] = [
    room("room-living-room", "Living room", RoomArea::Inside),
    room("room-kitchen", "Kitchen", RoomArea::Inside),
    room("room-bedroom", "Bedroom", RoomArea::Inside),
    room("room-bathroom", "Bathroom", RoomArea::Inside),
    room("room-entrance-hall", "Entrance hall", RoomArea::Inside),
    room("room-stairwell", "Stairwell (shared)", RoomArea::Common),
];

pub const STARTER_SYMPTOMS: [StarterSymptom; 6] = [
    symptom("symptom-cough", "Cough"),
    symptom("symptom-headache", "Headache"),
    symptom("symptom-sore-throat", "Sore throat"),
    symptom("symptom-eye-irritation", "Eye irritation"),
    symptom("symptom-nausea", "Nausea"),
    symptom("symptom-difficulty-sleeping", "Difficulty sleeping"),
];

/// Adds the starter rooms if there are no rooms and the starter symptoms if there are no
/// symptoms, through one audited transaction. Safe to run on every `migrate`.
pub async fn seed_starter_lists<S>(store: &S) -> StoreResult<()>
where
    S: Transactional<Tx: ListWrites> + ListRepo,
{
    let seed_rooms = store.rooms().await?.is_empty();
    let seed_symptoms = store.symptoms().await?.is_empty();
    if !seed_rooms && !seed_symptoms {
        return Ok(());
    }
    let mut tx = store.begin(None).await?;
    if seed_rooms {
        for room in &STARTER_ROOMS {
            tx.create_room(NewRoom {
                name: ListName::new(room.name)?,
                i18n_key: Some(room.i18n_key.to_owned()),
                area: room.area,
            })
            .await?;
        }
    }
    if seed_symptoms {
        for symptom in &STARTER_SYMPTOMS {
            tx.create_symptom(NewSymptom {
                name: ListName::new(symptom.name)?,
                i18n_key: Some(symptom.i18n_key.to_owned()),
            })
            .await?;
        }
    }
    tx.commit().await
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn the_starter_lists_match_the_spec() {
        assert_eq!(STARTER_ROOMS.len(), 6);
        assert_eq!(STARTER_SYMPTOMS.len(), 6);
        let common: Vec<_> = STARTER_ROOMS
            .iter()
            .filter(|r| r.area == RoomArea::Common)
            .map(|r| r.i18n_key)
            .collect();
        assert_eq!(common, ["room-stairwell"]);
    }

    #[test]
    fn starter_keys_are_unique_and_names_are_valid() {
        let rooms = STARTER_ROOMS.iter().map(|r| (r.i18n_key, r.name));
        let symptoms = STARTER_SYMPTOMS.iter().map(|s| (s.i18n_key, s.name));
        let items: Vec<_> = rooms.chain(symptoms).collect();
        let keys: HashSet<_> = items.iter().map(|(key, _)| key).collect();
        assert_eq!(keys.len(), 12);
        for (_, name) in items {
            assert!(ListName::new(name).is_ok(), "{name}");
        }
    }
}
