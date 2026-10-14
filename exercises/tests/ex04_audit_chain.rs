use smoke_exercises::ex04_audit_chain::{
    ChainError, Entry, GENESIS, append, build_chain, link_hash, verify_chain,
};

fn chain() -> Vec<Entry> {
    build_chain(&["created report", "corrected report", "closed event"])
}

#[test]
fn entries_are_numbered_from_one_and_linked_in_order() {
    let entries = chain();
    let ids: Vec<u32> = entries.iter().map(|e| e.id).collect();
    assert_eq!(ids, [1, 2, 3]);
    assert_eq!(entries[0].prev_hash, GENESIS);
    assert_eq!(entries[1].prev_hash, entries[0].hash);
    assert_eq!(entries[2].prev_hash, entries[1].hash);
    assert_eq!(
        entries[1].hash,
        link_hash(entries[0].hash, 2, "corrected report")
    );
}

#[test]
fn appending_extends_the_chain() {
    let mut entries = Vec::new();
    append(&mut entries, "first");
    append(&mut entries, "second");
    assert_eq!(entries, build_chain(&["first", "second"]));
}

#[test]
fn an_untouched_chain_verifies_and_so_does_an_empty_one() {
    assert_eq!(verify_chain(&chain()), Ok(()));
    assert_eq!(verify_chain(&[]), Ok(()));
}

#[test]
fn editing_a_payload_is_a_hash_mismatch_at_that_entry() {
    let mut entries = chain();
    entries[1].payload = "corrected report (edited)".to_owned();
    assert_eq!(
        verify_chain(&entries),
        Err(ChainError::HashMismatch { id: 2 })
    );
}

#[test]
fn removing_an_entry_breaks_the_link_after_it() {
    let mut entries = chain();
    entries.remove(1);
    assert_eq!(
        verify_chain(&entries),
        Err(ChainError::BrokenLink { id: 3 })
    );
}

#[test]
fn removing_the_first_entry_breaks_the_link_of_the_new_first() {
    let mut entries = chain();
    entries.remove(0);
    assert_eq!(
        verify_chain(&entries),
        Err(ChainError::BrokenLink { id: 2 })
    );
}

#[test]
fn recomputing_one_hash_hides_nothing_because_the_next_entry_still_points_at_the_old_one() {
    let mut entries = chain();
    entries[1].payload = "corrected report (edited)".to_owned();
    entries[1].hash = link_hash(entries[1].prev_hash, 2, &entries[1].payload);
    assert_eq!(
        verify_chain(&entries),
        Err(ChainError::BrokenLink { id: 3 })
    );
}

#[test]
fn a_tampered_last_entry_is_still_a_hash_mismatch() {
    let mut entries = chain();
    entries[2].payload = "closed event (edited)".to_owned();
    assert_eq!(
        verify_chain(&entries),
        Err(ChainError::HashMismatch { id: 3 })
    );
}
