//! Exercise 13: reading a form whose field names repeat. Borrowing and lifetimes.
//!
//! Practises: `docs/learning/04-logging-flow.md`, section 6.
//! Mirrors: `smoke_web::forms::Pairs`.
//! Run: `cargo test --test ex13_form_pairs`
//!
//! Hint: `Pairs` owns its `String`s; every method that returns text only *borrows* it from the
//! `Pairs`, so no copying is needed. `names_with_prefix` returns an iterator that borrows both
//! `self` and `prefix`, which is why both carry the same lifetime `'a`. `Iterator::filter_map`,
//! `str::strip_prefix` and collecting into `Option<Vec<_>>` (stops at the first `None`) will
//! shorten the code.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pairs(Vec<(String, String)>);

impl Pairs {
    /// Builds the pairs from `(name, value)` slices, keeping their order.
    pub fn of(pairs: &[(&str, &str)]) -> Self {
        todo!()
    }

    /// The first value of `name`, trimmed; `None` if the name is absent or its value blank.
    pub fn get(&self, name: &str) -> Option<&str> {
        todo!()
    }

    /// Every value of `name`, trimmed and parsed as an id, in order; `None` if one of them is not a number.
    /// A name that never appears gives `Some(vec![])`.
    pub fn ids(&self, name: &str) -> Option<Vec<i64>> {
        todo!()
    }

    /// What follows `prefix` in each field name that starts with it: the room ids of the
    /// `level-<id>` fields, for example.
    pub fn names_with_prefix<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        // The stub returns an empty iterator so that the test file compiles.
        std::iter::empty()
    }

    /// All pairs except those whose name is in `skip`: what a follow-up form re-posts as
    /// hidden fields.
    pub fn without(&self, skip: &[&str]) -> Vec<(String, String)> {
        todo!()
    }
}
