//! Exercise 6: a transaction that rolls back when it is dropped.
//!
//! Practises: `docs/learning/02-storage.md`, section 4.
//! Mirrors: `smoke_domain::repos::{Transactional, Commit}` and `smoke_storage_sqlite::SqliteTx`:
//! "dropping a transaction without `commit` rolls it back".
//! Run: `cargo test --test ex06_transaction`
//!
//! Hint: `Tx` holds an exclusive borrow of the `Ledger` (`&'a mut Ledger`), so nothing else can
//! touch the ledger while a transaction is alive, and the compiler enforces it. `commit` takes
//! `self` by value, so a committed transaction cannot be used again. Rust runs `Drop::drop`
//! when a value goes out of scope, also after `commit` consumed it: remember in `committed`
//! whether the commit happened, and count a rollback only when it did not.

#[derive(Debug, Default)]
pub struct Ledger {
    entries: Vec<String>,
    rollbacks: u32,
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }

    /// The committed entries.
    pub fn entries(&self) -> &[String] {
        todo!()
    }

    /// How many transactions ended without a commit.
    pub fn rollbacks(&self) -> u32 {
        todo!()
    }

    /// Starts a transaction. The ledger is borrowed mutably until it ends.
    pub fn begin(&mut self) -> Tx<'_> {
        todo!()
    }
}

pub struct Tx<'a> {
    ledger: &'a mut Ledger,
    pending: Vec<String>,
    committed: bool,
}

impl Tx<'_> {
    /// Buffers an entry; the ledger does not see it before `commit`.
    pub fn write(&mut self, entry: &str) {
        todo!()
    }

    /// What this transaction would add.
    pub fn pending(&self) -> &[String] {
        todo!()
    }

    /// Makes every buffered entry part of the ledger, in order.
    pub fn commit(self) {
        todo!()
    }
}

impl Drop for Tx<'_> {
    /// Without a commit: count one rollback on the ledger and discard the buffer.
    fn drop(&mut self) {
        // Write this: the stub does nothing.
    }
}

/// Writes every item in one transaction and commits, returning how many were written. An empty
/// item is refused with `Err("empty entry at index N")` before anything is committed, so
/// nothing from this call reaches the ledger.
pub fn write_all(ledger: &mut Ledger, items: &[&str]) -> Result<usize, String> {
    todo!()
}
