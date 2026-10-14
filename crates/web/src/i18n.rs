//! Translations (Fluent, one `.ftl` file per language, embedded at compile time) and the choice
//! of language for a request.

use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::Infallible;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use fluent_templates::fluent_bundle::FluentValue;
use fluent_templates::{LanguageIdentifier, Loader, langid, static_loader};
use smoke_domain::types::Locale;

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "en",
        customise: |bundle| bundle.set_use_isolating(false),
    };
}

static EN: LanguageIdentifier = langid!("en");
static IT: LanguageIdentifier = langid!("it");
static TR: LanguageIdentifier = langid!("tr");

fn language(locale: Locale) -> &'static LanguageIdentifier {
    match locale {
        Locale::En => &EN,
        Locale::It => &IT,
        Locale::Tr => &TR,
    }
}

/// Translates for one language. Templates call `tr.t("app-name")`; a key missing from a language
/// falls back to English.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tr(pub Locale);

impl Tr {
    pub(crate) fn lang(&self) -> &'static str {
        self.0.as_str()
    }

    pub(crate) fn t(&self, key: &str) -> String {
        LOCALES.lookup(language(self.0), key)
    }

    /// A message with one argument, e.g. `tr.t1("signed-in-as", "username", name)`.
    pub(crate) fn t1(&self, key: &str, name: &str, value: &str) -> String {
        let args: HashMap<Cow<'static, str>, FluentValue> = HashMap::from([(
            Cow::Owned(name.to_owned()),
            FluentValue::from(value.to_owned()),
        )]);
        LOCALES.lookup_with_args(language(self.0), key, &args)
    }

    /// A message that depends on a number, e.g. `tr.n("summary-events", &3)`: Fluent picks the
    /// language's plural form from `$count`. Takes a reference because templates pass one.
    pub(crate) fn n(&self, key: &str, count: &usize) -> String {
        let args: HashMap<Cow<'static, str>, FluentValue> =
            HashMap::from([(Cow::Borrowed("count"), FluentValue::from(*count))]);
        LOCALES.lookup_with_args(language(self.0), key, &args)
    }
}

/// The language chosen by [`crate::auth::load_user`]; English if that middleware did not run.
impl<S: Send + Sync> FromRequestParts<S> for Tr {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<Tr>()
            .copied()
            .unwrap_or(Tr(Locale::En)))
    }
}

/// The cookie that remembers the language before sign-in (spec §9.2).
pub(crate) const LANG_COOKIE: &str = "lang";

/// The account's language when signed in, else the `lang` cookie, else the browser's
/// `Accept-Language`, else English. Unknown values are skipped, never an error.
pub(crate) fn negotiate(
    account: Option<Locale>,
    cookie: Option<&str>,
    accept_language: Option<&str>,
) -> Locale {
    account
        .or_else(|| cookie.and_then(|c| c.trim().parse().ok()))
        .or_else(|| accept_language.and_then(from_accept_language))
        .unwrap_or(Locale::En)
}

/// The supported language with the highest quality; the first one listed wins a tie.
fn from_accept_language(header: &str) -> Option<Locale> {
    let mut best: Option<(f32, Locale)> = None;
    for item in header.split(',') {
        let mut parts = item.split(';');
        let tag = parts.next().unwrap_or_default().trim();
        let quality = parts
            .find_map(|p| p.trim().strip_prefix("q="))
            .map_or(Some(1.0), |q| q.trim().parse::<f32>().ok())
            .unwrap_or(0.0);
        let primary = tag
            .split('-')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if let Ok(locale) = primary.parse::<Locale>()
            && quality > 0.0
            && best.is_none_or(|(q, _)| quality > q)
        {
            best = Some((quality, locale));
        }
    }
    best.map(|(_, locale)| locale)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;

    use fluent_syntax::ast::Entry;
    use smoke_domain::starter::{STARTER_ROOMS, STARTER_SYMPTOMS};

    use super::*;
    use crate::routes::login::Notice;
    use crate::routes::{self, entries, graphs, guide, lists, people, reports, settings};
    use crate::{error, labels};

    const DIR: &str = env!("CARGO_MANIFEST_DIR");

    fn message_ids(locale: Locale) -> BTreeSet<String> {
        let path = format!("{DIR}/locales/{locale}/main.ftl");
        let source = fs::read_to_string(&path).unwrap();
        let resource = fluent_syntax::parser::parse(source.as_str())
            .unwrap_or_else(|(_, errors)| panic!("{path}: {errors:?}"));
        resource
            .body
            .iter()
            .filter_map(|entry| match entry {
                Entry::Message(message) => Some(message.id.name.to_owned()),
                _ => None,
            })
            .collect()
    }

    /// Every string literal passed as the key to `tr.t`, `tr.t1` or `tr.n` in the templates
    /// and the Rust sources.
    fn literal_keys() -> BTreeSet<String> {
        fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    files(&path, out);
                } else {
                    out.push(path);
                }
            }
        }
        let mut paths = Vec::new();
        for dir in ["templates", "src"] {
            files(std::path::Path::new(&format!("{DIR}/{dir}")), &mut paths);
        }
        let mut keys = BTreeSet::new();
        for path in paths {
            let text = fs::read_to_string(path).unwrap();
            for call in ["tr.t(\"", "tr.t1(\"", "tr.n(\""] {
                for (start, _) in text.match_indices(call) {
                    let rest = &text[start + call.len()..];
                    keys.insert(rest[..rest.find('"').unwrap()].to_owned());
                }
            }
        }
        keys
    }

    #[test]
    fn every_language_has_exactly_the_english_keys() {
        let english = message_ids(Locale::En);
        for locale in Locale::ALL {
            let ids = message_ids(*locale);
            let missing: Vec<_> = english.difference(&ids).collect();
            let extra: Vec<_> = ids.difference(&english).collect();
            assert!(
                missing.is_empty() && extra.is_empty(),
                "{locale}: missing {missing:?}, not in English {extra:?}"
            );
        }
    }

    #[test]
    fn the_starter_lists_are_translated() {
        let english = message_ids(Locale::En);
        let keys = STARTER_ROOMS
            .iter()
            .map(|room| room.i18n_key)
            .chain(STARTER_SYMPTOMS.iter().map(|symptom| symptom.i18n_key));
        for key in keys {
            assert!(english.contains(key), "{key} has no translation");
        }
    }

    #[test]
    fn every_key_the_pages_use_is_translated() {
        let mut used = literal_keys();
        assert!(used.contains("app-name"), "the template scan found nothing");
        assert!(
            used.contains("prompt-same-smell"),
            "the source scan found nothing"
        );
        used.extend(error::KEYS.map(str::to_owned));
        used.extend(Notice::ALL.map(|notice| notice.key().to_owned()));
        used.extend(guide::keys());
        used.extend(labels::keys().into_iter().map(str::to_owned));
        used.extend(entries::FORM_ERRORS.map(str::to_owned));
        used.extend(routes::REFUSALS.map(str::to_owned));
        used.extend(reports::HISTORY_KEYS.map(str::to_owned));
        used.extend(settings::keys().into_iter().map(str::to_owned));
        used.extend(lists::KEYS.map(str::to_owned));
        used.extend(people::keys().into_iter().map(str::to_owned));
        used.extend(graphs::KEYS.map(str::to_owned));
        let english = message_ids(Locale::En);
        let missing: Vec<_> = used.difference(&english).collect();
        assert!(
            missing.is_empty(),
            "keys without a translation: {missing:?}"
        );
    }

    #[test]
    fn there_is_a_folder_for_each_supported_language_and_no_other() {
        let folders: BTreeSet<String> = fs::read_dir(format!("{DIR}/locales"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        let supported: BTreeSet<String> =
            Locale::ALL.iter().map(|l| l.as_str().to_owned()).collect();
        assert_eq!(folders, supported);
    }

    #[test]
    fn the_account_language_wins_then_the_cookie_then_the_browser() {
        assert_eq!(
            negotiate(Some(Locale::Tr), Some("it"), Some("en")),
            Locale::Tr
        );
        assert_eq!(negotiate(None, Some("it"), Some("tr")), Locale::It);
        assert_eq!(negotiate(None, None, Some("tr")), Locale::Tr);
        assert_eq!(negotiate(None, None, None), Locale::En);
    }

    #[test]
    fn unusable_values_fall_through_to_the_next_source() {
        assert_eq!(negotiate(None, Some("fr"), Some("it-IT")), Locale::It);
        assert_eq!(
            negotiate(None, Some(""), Some("de-DE,fr;q=0.9")),
            Locale::En
        );
        assert_eq!(negotiate(None, Some("IT"), None), Locale::En);
    }

    #[test]
    fn accept_language_uses_the_primary_tag_and_quality() {
        assert_eq!(
            from_accept_language("it-IT,it;q=0.9,en;q=0.8"),
            Some(Locale::It)
        );
        assert_eq!(from_accept_language("tr-TR"), Some(Locale::Tr));
        assert_eq!(
            from_accept_language("de-DE, en;q=0.5, it;q=0.8"),
            Some(Locale::It)
        );
        assert_eq!(from_accept_language("EN-us"), Some(Locale::En));
        assert_eq!(from_accept_language("it;q=0, tr;q=0.1"), Some(Locale::Tr));
        assert_eq!(from_accept_language("en, it"), Some(Locale::En));
        assert_eq!(from_accept_language("*"), None);
        assert_eq!(from_accept_language("it;q=abc"), None);
        assert_eq!(from_accept_language(""), None);
    }

    #[test]
    fn messages_are_translated_and_take_arguments() {
        assert_eq!(Tr(Locale::En).t("login-title"), "Sign in");
        assert_eq!(Tr(Locale::It).t("login-title"), "Accedi");
        assert_eq!(Tr(Locale::Tr).t("login-title"), "Giriş yap");
        assert_eq!(
            Tr(Locale::En).t1("signed-in-as", "username", "daniele"),
            "Signed in as daniele."
        );
    }

    #[test]
    fn counts_pick_the_plural_form_of_each_language() {
        assert_eq!(Tr(Locale::En).n("summary-events", &1), "1 event");
        assert_eq!(Tr(Locale::En).n("summary-events", &14), "14 events");
        assert_eq!(Tr(Locale::It).n("summary-reports", &1), "1 segnalazione");
        assert_eq!(Tr(Locale::It).n("summary-reports", &41), "41 segnalazioni");
        assert_eq!(Tr(Locale::Tr).n("summary-events", &14), "14 olay");
    }
}
