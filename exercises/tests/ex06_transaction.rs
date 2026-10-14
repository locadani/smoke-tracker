use smoke_exercises::ex06_transaction::{Ledger, write_all};

#[test]
fn a_new_ledger_is_empty() {
    let ledger = Ledger::new();
    assert!(ledger.entries().is_empty());
    assert_eq!(ledger.rollbacks(), 0);
}

#[test]
fn commit_adds_the_entries_in_order() {
    let mut ledger = Ledger::new();
    let mut tx = ledger.begin();
    tx.write("a");
    tx.write("b");
    tx.commit();
    assert_eq!(ledger.entries(), ["a", "b"]);
    assert_eq!(ledger.rollbacks(), 0);
}

#[test]
fn entries_wait_in_the_transaction_until_commit() {
    let mut ledger = Ledger::new();
    let mut tx = ledger.begin();
    tx.write("a");
    assert_eq!(tx.pending(), ["a"]);
    tx.commit();
    assert_eq!(ledger.entries(), ["a"]);
}

#[test]
fn dropping_without_commit_discards_and_counts_a_rollback() {
    let mut ledger = Ledger::new();
    {
        let mut tx = ledger.begin();
        tx.write("lost");
    }
    assert!(ledger.entries().is_empty());
    assert_eq!(ledger.rollbacks(), 1);
}

#[test]
fn a_rolled_back_transaction_leaves_earlier_commits_alone() {
    let mut ledger = Ledger::new();
    let mut first = ledger.begin();
    first.write("kept");
    first.commit();
    drop({
        let mut second = ledger.begin();
        second.write("lost");
        second
    });
    let mut third = ledger.begin();
    third.write("also kept");
    third.commit();
    assert_eq!(ledger.entries(), ["kept", "also kept"]);
    assert_eq!(ledger.rollbacks(), 1);
}

#[test]
fn write_all_commits_everything_or_nothing() {
    let mut ledger = Ledger::new();
    assert_eq!(write_all(&mut ledger, &["a", "b", "c"]), Ok(3));
    assert_eq!(ledger.entries(), ["a", "b", "c"]);

    assert_eq!(
        write_all(&mut ledger, &["d", "e", "", "f"]),
        Err("empty entry at index 2".to_owned())
    );
    assert_eq!(ledger.entries(), ["a", "b", "c"]);
    assert_eq!(ledger.rollbacks(), 1);
}

#[test]
fn write_all_with_nothing_to_write_commits_nothing() {
    let mut ledger = Ledger::new();
    assert_eq!(write_all(&mut ledger, &[]), Ok(0));
    assert!(ledger.entries().is_empty());
    assert_eq!(ledger.rollbacks(), 0);
}
