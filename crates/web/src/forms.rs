//! Urlencoded forms and query strings whose names repeat (`room=1&room=3`): `serde_urlencoded`
//! cannot put those in a struct, so handlers read the pairs instead.

use std::convert::Infallible;

use axum::Form;
use axum::body::{Body, to_bytes};
use axum::extract::rejection::FormRejection;
use axum::extract::{FromRequest, FromRequestParts, Multipart, Query, Request};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};

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

    /// Whether `name` is there at all, even blank (`?welcome`).
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.0.iter().any(|(key, _)| key == name)
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

    /// Sets `name` to `value`, replacing any earlier values.
    pub(crate) fn set(&mut self, name: &str, value: String) {
        self.0.retain(|(key, _)| key != name);
        self.0.push((name.to_owned(), value));
    }

    /// The pairs without any of the fields in `names`.
    pub(crate) fn omitting(mut self, names: &[&str]) -> Self {
        self.0.retain(|(key, _)| !names.contains(&key.as_str()));
        self
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

/// The Log form as posted: its fields and, when the browser sent one, the `photo` file. A
/// `multipart/form-data` post carries both; a urlencoded post is read as before, capped at the
/// ordinary form limit even on routes that accept a larger upload.
#[derive(Debug, Default)]
pub(crate) struct Upload {
    pub(crate) pairs: Pairs,
    /// The bytes of the `photo` part; `None` when absent or empty (a browser sends an empty
    /// part when no file was chosen).
    pub(crate) photo: Option<Vec<u8>>,
}

impl<S: Send + Sync> FromRequest<S> for Upload {
    type Rejection = Response;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let multipart = request
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("multipart/form-data"));
        if !multipart {
            let (parts, body) = request.into_parts();
            let bytes = to_bytes(body, crate::BODY_LIMIT_BYTES)
                .await
                .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE.into_response())?;
            let request = Request::from_parts(parts, Body::from(bytes));
            let pairs = Pairs::from_request(request, state)
                .await
                .map_err(IntoResponse::into_response)?;
            return Ok(Self { pairs, photo: None });
        }
        let mut parts = Multipart::from_request(request, state)
            .await
            .map_err(IntoResponse::into_response)?;
        let mut upload = Self::default();
        while let Some(field) = parts
            .next_field()
            .await
            .map_err(IntoResponse::into_response)?
        {
            let name = field.name().unwrap_or_default().to_owned();
            if name == "photo" {
                let bytes = field.bytes().await.map_err(IntoResponse::into_response)?;
                upload.photo = (!bytes.is_empty()).then(|| bytes.to_vec());
            } else {
                let text = field.text().await.map_err(IntoResponse::into_response)?;
                upload.pairs.0.push((name, text));
            }
        }
        Ok(upload)
    }
}

/// The query string as its pairs. Unlike a typed `Query`, a repeated name (`?month=1&month=2`)
/// or one that does not parse never answers 400: the page reads the first value, or none, and
/// falls back to its default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct QueryPairs(pub(crate) Pairs);

impl<S: Send + Sync> FromRequestParts<S> for QueryPairs {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let pairs = Query::<Vec<(String, String)>>::try_from_uri(&parts.uri)
            .map(|Query(pairs)| pairs)
            .unwrap_or_default();
        Ok(Self(Pairs(pairs)))
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
