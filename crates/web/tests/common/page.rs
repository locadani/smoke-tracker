//! A rendered page, queried with CSS selectors, so tests assert what the page means rather than
//! how its markup happens to be spelled.

use scraper::{ElementRef, Html, Selector};

pub struct Page(Html);

fn selector(css: &str) -> Selector {
    Selector::parse(css).unwrap_or_else(|error| panic!("bad selector {css}: {error}"))
}

/// An element's text with runs of whitespace collapsed to one space.
fn text_of(element: ElementRef) -> String {
    element
        .text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

impl Page {
    pub fn parse(html: &str) -> Self {
        Self(Html::parse_document(html))
    }

    pub fn exists(&self, css: &str) -> bool {
        self.0.select(&selector(css)).next().is_some()
    }

    pub fn count(&self, css: &str) -> usize {
        self.0.select(&selector(css)).count()
    }

    /// The text of every match, in document order.
    pub fn texts(&self, css: &str) -> Vec<String> {
        self.0.select(&selector(css)).map(text_of).collect()
    }

    /// The text of the first match; panics when nothing matches.
    pub fn text(&self, css: &str) -> String {
        self.texts(css)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("nothing matches {css}"))
    }

    /// The attribute `name` of every match that has it.
    pub fn attrs(&self, css: &str, name: &str) -> Vec<String> {
        self.0
            .select(&selector(css))
            .filter_map(|element| element.value().attr(name).map(str::to_owned))
            .collect()
    }

    /// The attribute `name` of the first match.
    pub fn attr(&self, css: &str, name: &str) -> Option<String> {
        self.attrs(css, name).into_iter().next()
    }
}
