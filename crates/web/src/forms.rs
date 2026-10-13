//! Urlencoded forms whose field names repeat (`room=1&room=3`): `serde_urlencoded` cannot put
//! those in a struct, so handlers read the pairs instead.

use axum::Form;
use axum::extract::rejection::FormRejection;
use axum::extract::{FromRequest, Request};

/// A posted form as its `(name, value)` pairs, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Pairs(Vec<(String, String)>);

impl Pairs {
    #[cfg(test)]
    pub(crate) fn of(pairs: &[(&str, &str)]) -> Self {
        Self(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
        )
    }

    /// The first value of `name`, trimmed; `None` if absent or blank.
    pub(crate) fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.trim())
            .filter(|value| !value.is_empty())
    }

    /// Every value of `name` parsed as an id; `None` if one is not a number.
    pub(crate) fn ids(&self, name: &str) -> Option<Vec<i64>> {
        self.0
            .iter()
            .filter(|(key, _)| key == name)
            .map(|(_, value)| value.trim().parse().ok())
            .collect()
    }

    /// What follows `prefix` in each field name that starts with it, e.g. the room ids of
    /// `level-<id>` fields.
    pub(crate) fn names_with_prefix<'a>(
        &'a self,
        prefix: &'a str,
    ) -> impl Iterator<Item = &'a str> + 'a {
        self.0
            .iter()
            .filter_map(move |(key, _)| key.strip_prefix(prefix))
    }

    /// All pairs except those named in `skip`: what a follow-up form re-posts as hidden fields.
    pub(crate) fn without(&self, skip: &[&str]) -> Vec<(String, String)> {
        self.0
            .iter()
            .filter(|(key, _)| !skip.contains(&key.as_str()))
            .cloned()
            .collect()
    }
}

impl<S: Send + Sync> FromRequest<S> for Pairs {
    type Rejection = FormRejection;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Form(pairs) = Form::<Vec<(String, String)>>::from_request(request, state).await?;
        Ok(Self(pairs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_names_are_all_kept() {
        let form = Pairs::of(&[
            ("intensity", " 6 "),
            ("room", "3"),
            ("room", "1"),
            ("note", "  "),
        ]);
        assert_eq!(form.get("intensity"), Some("6"));
        assert_eq!(form.get("note"), None);
        assert_eq!(form.get("missing"), None);
        assert_eq!(form.ids("room"), Some(vec![3, 1]));
        assert_eq!(form.ids("symptom"), Some(vec![]));
        assert_eq!(Pairs::of(&[("room", "x")]).ids("room"), None);
        assert_eq!(
            form.without(&["room", "note"]),
            [("intensity".to_owned(), " 6 ".to_owned())]
        );
    }
}
