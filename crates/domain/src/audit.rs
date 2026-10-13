//! The tamper-evident audit log: each entry's hash covers its content and the previous hash,
//! so changing or removing any entry breaks every link after it.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::types::{UserId, str_enum};

str_enum!(
    /// What happened to an entity.
    AuditAction {
        Create => "create",
        Update => "update",
        Delete => "delete",
        Close => "close",
        Reopen => "reopen",
        Move => "move",
        Invite => "invite",
        Expire => "expire",
        Reactivate => "reactivate",
        PasswordChange => "password_change",
    }
);

/// The `prev_hash` of the first entry.
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// One audit entry's content: everything the hash covers.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AuditRecord {
    pub id: i64,
    /// Microseconds since the Unix epoch: the finest precision both SQLite and Postgres keep.
    pub at_micros: i64,
    pub actor_id: Option<UserId>,
    pub entity: String,
    pub entity_id: i64,
    pub action: AuditAction,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

/// SHA-256 over `prev_hash`, a newline, and the record as canonical JSON, as lowercase hex.
pub fn compute_hash(record: &AuditRecord, prev_hash: &str) -> String {
    let value = serde_json::to_value(record).expect("AuditRecord always serialises");
    let mut bytes = Vec::new();
    write_canonical(&value, &mut bytes);
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(b"\n");
    hasher.update(&bytes);
    hex::encode(hasher.finalize())
}

/// Compact JSON with object keys sorted at every level, written explicitly so the bytes never
/// depend on how serde_json's map type is configured by Cargo feature unification.
fn write_canonical(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_canonical(item, out);
            }
            out.push(b']');
        }
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
            out.push(b'{');
            for (i, (key, item)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                serde_json::to_writer(&mut *out, key).expect("writing to a Vec cannot fail");
                out.push(b':');
                write_canonical(item, out);
            }
            out.push(b'}');
        }
        scalar => serde_json::to_writer(&mut *out, scalar).expect("writing to a Vec cannot fail"),
    }
}

/// A record together with its place in the chain.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainedEntry {
    pub record: AuditRecord,
    pub prev_hash: String,
    pub hash: String,
}

impl ChainedEntry {
    /// Computes the hash and links the record after `prev_hash`.
    pub fn seal(record: AuditRecord, prev_hash: &str) -> Self {
        let hash = compute_hash(&record, prev_hash);
        Self {
            record,
            prev_hash: prev_hash.to_owned(),
            hash,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChainError {
    #[error("audit entry {id}: its prev_hash does not match the previous entry's hash")]
    BrokenLink { id: i64 },
    #[error("audit entry {id}: its content does not match its hash")]
    HashMismatch { id: i64 },
}

/// Checks the whole chain from the genesis hash and reports the first problem found.
pub fn verify_chain(entries: &[ChainedEntry]) -> Result<(), ChainError> {
    let mut expected_prev = GENESIS_HASH;
    for entry in entries {
        let id = entry.record.id;
        if entry.prev_hash != expected_prev {
            return Err(ChainError::BrokenLink { id });
        }
        if compute_hash(&entry.record, &entry.prev_hash) != entry.hash {
            return Err(ChainError::HashMismatch { id });
        }
        expected_prev = &entry.hash;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use proptest::prelude::*;
    use proptest::sample::Index;
    use serde_json::{Map, json};

    use super::*;

    fn record(id: i64, after: Value) -> AuditRecord {
        AuditRecord {
            id,
            at_micros: 1_768_500_000_000_000 + id,
            actor_id: Some(UserId(1)),
            entity: "smell_report".into(),
            entity_id: 12,
            action: AuditAction::Create,
            before: None,
            after: Some(after),
        }
    }

    fn build_chain(n: usize) -> Vec<ChainedEntry> {
        let mut chain: Vec<ChainedEntry> = Vec::with_capacity(n);
        for i in 0..n {
            let prev = chain
                .last()
                .map_or(GENESIS_HASH, |e| e.hash.as_str())
                .to_owned();
            let id = i as i64 + 1;
            chain.push(ChainedEntry::seal(
                record(id, json!({ "intensity": (i % 10) + 1 })),
                &prev,
            ));
        }
        chain
    }

    #[test]
    fn genesis_hash_is_sixty_four_zeros() {
        assert_eq!(GENESIS_HASH.len(), 64);
        assert!(GENESIS_HASH.chars().all(|c| c == '0'));
    }

    #[test]
    fn hash_is_deterministic_lowercase_hex() {
        let r = record(1, json!({ "intensity": 6 }));
        let a = compute_hash(&r, GENESIS_HASH);
        assert_eq!(a, compute_hash(&r, GENESIS_HASH));
        assert_eq!(a.len(), 64);
        assert!(
            a.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn hash_depends_on_the_previous_hash() {
        let r = record(1, json!({ "intensity": 6 }));
        assert_ne!(
            compute_hash(&r, GENESIS_HASH),
            compute_hash(&r, &"f".repeat(64))
        );
    }

    /// Guards against serde_json's `preserve_order` feature being switched on by any
    /// dependency (Cargo unifies features): key order must never affect the hash.
    #[test]
    fn json_key_order_does_not_change_the_hash() {
        let mut za = Map::new();
        za.insert("z".into(), json!(1));
        za.insert("a".into(), json!(2));
        let mut az = Map::new();
        az.insert("a".into(), json!(2));
        az.insert("z".into(), json!(1));
        assert_eq!(
            compute_hash(&record(1, Value::Object(za)), GENESIS_HASH),
            compute_hash(&record(1, Value::Object(az)), GENESIS_HASH)
        );
    }

    #[test]
    fn canonical_form_is_compact_with_keys_sorted_at_every_level() {
        let value = json!({ "z": [1, { "b": true, "a": null }], "a": "x\"y", "m": 2.5 });
        let mut out = Vec::new();
        write_canonical(&value, &mut out);
        assert_eq!(
            String::from_utf8(out).unwrap(),
            r#"{"a":"x\"y","m":2.5,"z":[1,{"a":null,"b":true}]}"#
        );
    }

    /// Postgres keeps microseconds; anything finer must not reach the hash.
    #[test]
    fn sub_microsecond_precision_does_not_change_the_hash() {
        let precise: DateTime<Utc> = "2026-01-15T20:00:00.123456789Z".parse().unwrap();
        let as_stored_by_postgres: DateTime<Utc> = "2026-01-15T20:00:00.123456Z".parse().unwrap();
        let mut a = record(1, json!({}));
        let mut b = record(1, json!({}));
        a.at_micros = precise.timestamp_micros();
        b.at_micros = as_stored_by_postgres.timestamp_micros();
        assert_eq!(
            compute_hash(&a, GENESIS_HASH),
            compute_hash(&b, GENESIS_HASH)
        );
    }

    #[test]
    fn an_empty_chain_and_a_valid_chain_verify() {
        assert_eq!(verify_chain(&[]), Ok(()));
        assert_eq!(verify_chain(&build_chain(5)), Ok(()));
    }

    #[test]
    fn editing_an_entry_is_reported_at_that_entry() {
        let mut chain = build_chain(3);
        chain[0].record.after = Some(json!({ "intensity": 9 }));
        assert_eq!(
            verify_chain(&chain),
            Err(ChainError::HashMismatch { id: 1 })
        );
    }

    #[test]
    fn re_hashing_an_edited_entry_breaks_the_next_link() {
        let mut chain = build_chain(3);
        chain[0].record.after = Some(json!({ "intensity": 9 }));
        chain[0].hash = compute_hash(&chain[0].record, &chain[0].prev_hash);
        assert_eq!(verify_chain(&chain), Err(ChainError::BrokenLink { id: 2 }));
    }

    #[test]
    fn deleting_an_entry_breaks_the_chain() {
        let mut chain = build_chain(3);
        chain.remove(1);
        assert_eq!(verify_chain(&chain), Err(ChainError::BrokenLink { id: 3 }));
    }

    #[test]
    fn audit_actions_use_snake_case_text() {
        assert_eq!(AuditAction::PasswordChange.as_str(), "password_change");
        assert_eq!("reopen".parse::<AuditAction>(), Ok(AuditAction::Reopen));
    }

    proptest! {
        #[test]
        fn any_change_to_any_entry_is_detected(
            n in 1usize..20,
            pick in any::<Index>(),
            new_entity_id in any::<i64>(),
        ) {
            let mut chain = build_chain(n);
            let k = pick.index(n);
            prop_assume!(chain[k].record.entity_id != new_entity_id);
            chain[k].record.entity_id = new_entity_id;
            prop_assert_eq!(
                verify_chain(&chain),
                Err(ChainError::HashMismatch { id: chain[k].record.id })
            );
        }
    }
}
