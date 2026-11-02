use std::ops::RangeInclusive;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use serde_json::json;

use crate::GoogleAuth;

const VISION_URL: &str = "https://eu-vision.googleapis.com/v1/images:annotate";
const READ_DEADLINE: Duration = Duration::from_secs(10);
const MAX_PM25: f64 = 1000.0;

/// Reads the PM2.5 number from a sensor photo. A reader never fails: no number is `None`.
#[derive(Debug)]
pub enum Pm25Reader {
    None,
    CloudVision(GoogleAuth),
}

impl Pm25Reader {
    pub async fn read(&self, jpeg: &[u8]) -> Option<f64> {
        match self {
            Self::None => None,
            Self::CloudVision(auth) => {
                read_with_vision(auth, VISION_URL, jpeg, READ_DEADLINE).await
            }
        }
    }
}

/// One word found by Vision, with the corners of its box.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(from = "RawAnnotation")]
pub struct TextAnnotation {
    pub description: String,
    pub bounding_poly: Vec<(i32, i32)>,
}

#[derive(Deserialize)]
struct RawAnnotation {
    #[serde(default)]
    description: String,
    #[serde(default, rename = "boundingPoly")]
    bounding_poly: RawPoly,
}

#[derive(Default, Deserialize)]
struct RawPoly {
    #[serde(default)]
    vertices: Vec<RawVertex>,
}

#[derive(Deserialize)]
struct RawVertex {
    #[serde(default)]
    x: i32,
    #[serde(default)]
    y: i32,
}

impl From<RawAnnotation> for TextAnnotation {
    fn from(raw: RawAnnotation) -> Self {
        Self {
            description: raw.description,
            bounding_poly: raw
                .bounding_poly
                .vertices
                .into_iter()
                .map(|v| (v.x, v.y))
                .collect(),
        }
    }
}

#[derive(Deserialize)]
struct AnnotateResponse {
    #[serde(default)]
    responses: Vec<SingleResponse>,
}

#[derive(Deserialize)]
struct SingleResponse {
    #[serde(default, rename = "textAnnotations")]
    text_annotations: Vec<TextAnnotation>,
}

struct Bounds {
    x: RangeInclusive<i32>,
    y: RangeInclusive<i32>,
}

impl Bounds {
    fn of(word: &TextAnnotation) -> Option<Self> {
        let xs = word.bounding_poly.iter().map(|p| p.0);
        let ys = word.bounding_poly.iter().map(|p| p.1);
        Some(Self {
            x: xs.clone().min()?..=xs.max()?,
            y: ys.clone().min()?..=ys.max()?,
        })
    }

    fn union(&self, other: &Self) -> Self {
        Self {
            x: *self.x.start().min(other.x.start())..=*self.x.end().max(other.x.end()),
            y: *self.y.start().min(other.y.start())..=*self.y.end().max(other.y.end()),
        }
    }

    fn shares_a_band_with(&self, other: &Self) -> bool {
        self.y.start() <= other.y.end() && other.y.start() <= self.y.end()
    }

    fn is_right_of(&self, other: &Self) -> bool {
        self.x.start() >= other.x.end()
    }
}

/// The PM2.5 value among Vision's word-level annotations (without the whole-text one).
///
/// With a PM2.5 label, the nearest number to its right on the same band wins; without one, a
/// lone number does. Anything else is no number.
pub fn pick_pm25(text_annotations: &[TextAnnotation]) -> Option<f64> {
    let (label, label_words) = find_label(text_annotations);
    let numbers = text_annotations
        .iter()
        .enumerate()
        .filter(|(i, _)| !label_words.contains(i))
        .filter_map(|(_, word)| Some((parse_number(&word.description)?, Bounds::of(word)?)));
    match label {
        Some(label) => numbers
            .filter(|(_, b)| b.is_right_of(&label) && b.shares_a_band_with(&label))
            .min_by_key(|(_, b)| b.x.start() - label.x.end())
            .map(|(value, _)| value),
        None => {
            let mut numbers = numbers;
            let only = numbers.next()?;
            numbers.next().is_none().then_some(only.0)
        }
    }
}

fn find_label(words: &[TextAnnotation]) -> (Option<Bounds>, Vec<usize>) {
    for (i, word) in words.iter().enumerate() {
        let Some(bounds) = Bounds::of(word) else {
            continue;
        };
        if is_label(&word.description) {
            return (Some(bounds), vec![i]);
        }
        if word.description.eq_ignore_ascii_case("PM")
            && let Some((j, tail)) = words.iter().enumerate().find_map(|(j, w)| {
                let b = Bounds::of(w)?;
                (j != i
                    && matches!(w.description.as_str(), "2.5" | "2,5" | "25")
                    && b.is_right_of(&bounds)
                    && b.shares_a_band_with(&bounds))
                .then_some((j, b))
            })
        {
            return (Some(bounds.union(&tail)), vec![i, j]);
        }
    }
    (None, Vec::new())
}

fn is_label(text: &str) -> bool {
    let text = text.to_ascii_uppercase();
    ["PM2.5", "PM2,5", "PM25"].iter().any(|l| text.contains(l))
}

fn parse_number(text: &str) -> Option<f64> {
    let (whole, fraction) = match text.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (text, None),
    };
    let digits =
        |s: &str, max: usize| (1..=max).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole, 4) || fraction.is_some_and(|f| !digits(f, 1)) {
        return None;
    }
    text.parse().ok().filter(|v| (0.0..=MAX_PM25).contains(v))
}

fn words_from_response(body: &str) -> Vec<TextAnnotation> {
    serde_json::from_str::<AnnotateResponse>(body)
        .ok()
        .and_then(|r| r.responses.into_iter().next())
        .map(|r| r.text_annotations.into_iter().skip(1).collect())
        .unwrap_or_default()
}

async fn read_with_vision(
    auth: &GoogleAuth,
    endpoint: &str,
    jpeg: &[u8],
    deadline: Duration,
) -> Option<f64> {
    let outcome = tokio::time::timeout(deadline, annotate(auth, endpoint, jpeg))
        .await
        .unwrap_or_else(|_| Err("the whole read took too long".to_owned()));
    match outcome {
        Ok(body) => pick_pm25(&words_from_response(&body)),
        Err(reason) => {
            tracing::warn!(%reason, "could not read PM2.5 from the photo");
            None
        }
    }
}

async fn annotate(auth: &GoogleAuth, endpoint: &str, jpeg: &[u8]) -> Result<String, String> {
    let token = auth
        .token()
        .await
        .map_err(|_| "no access token".to_owned())?;
    let request = json!({"requests": [{
        "image": {"content": STANDARD.encode(jpeg)},
        "features": [{"type": "TEXT_DETECTION"}],
    }]});
    let response = auth
        .client()
        .post(endpoint)
        .bearer_auth(token)
        .json(&request)
        .send()
        .await
        .map_err(|_| "vision request failed".to_owned())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("vision answered with status {}", status.as_u16()));
    }
    response
        .text()
        .await
        .map_err(|_| "vision response unreadable".to_owned())
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    fn pick(fixture: &str) -> Option<f64> {
        let path = format!("{}/tests/fixtures/{fixture}", env!("CARGO_MANIFEST_DIR"));
        let body = std::fs::read_to_string(path).unwrap();
        pick_pm25(&words_from_response(&body))
    }

    #[test]
    fn label_and_number() {
        assert_eq!(pick("vision-label_and_number.json"), Some(19.0));
    }

    #[test]
    fn label_split_into_pm_and_2_5() {
        assert_eq!(pick("vision-label_split.json"), Some(31.0));
    }

    #[test]
    fn only_a_number() {
        assert_eq!(pick("vision-only_a_number.json"), Some(14.0));
    }

    #[test]
    fn two_numbers_without_label() {
        assert_eq!(pick("vision-two_numbers_without_label.json"), None);
    }

    #[test]
    fn label_without_a_number_to_its_right() {
        assert_eq!(pick("vision-label_without_number_to_the_right.json"), None);
    }

    #[test]
    fn nothing_readable() {
        assert_eq!(pick("vision-nothing_readable.json"), None);
        assert_eq!(pick("vision-empty.json"), None);
    }

    #[test]
    fn a_time_is_not_a_number() {
        assert_eq!(pick("vision-a_time.json"), None);
    }

    #[tokio::test]
    async fn a_reader_failure_is_just_no_number() {
        let auth = GoogleAuth::with_fixed_token("test-token").unwrap();
        let started = Instant::now();
        let number = read_with_vision(&auth, "http://127.0.0.1:9/", b"jpeg", READ_DEADLINE).await;
        assert_eq!(number, None);
        assert!(started.elapsed() < Duration::from_secs(11));
    }

    #[tokio::test]
    async fn a_silent_server_is_given_up_on_at_the_deadline() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/", listener.local_addr().unwrap());
        let _silent = tokio::spawn(async move {
            let _held = listener.accept().await;
            std::future::pending::<()>().await;
        });
        let auth = GoogleAuth::with_fixed_token("test-token").unwrap();
        let started = Instant::now();
        let number = read_with_vision(&auth, &endpoint, b"jpeg", Duration::from_millis(300)).await;
        assert_eq!(number, None);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn the_none_reader_reads_nothing() {
        assert_eq!(Pm25Reader::None.read(b"jpeg").await, None);
    }
}
