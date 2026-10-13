//! QR codes as inline SVG (spec §8.3). The page embeds the markup directly: the
//! Content-Security-Policy allows no `data:` images, and a link's token exists only while the
//! page that shows it is rendered.

use std::fmt::Write;

use qrcodegen::{QrCode, QrCodeEcc};

/// The light margin around the code, in modules, as the QR specification asks.
const QUIET_ZONE: i32 = 4;

/// An SVG drawing of `text` as a QR code with medium error correction, one square per dark
/// module. Colours are presentation attributes, so no style is needed.
pub(crate) fn svg(text: &str) -> String {
    let code =
        QrCode::encode_text(text, QrCodeEcc::Medium).expect("a link is far below the QR capacity");
    let side = code.size() + 2 * QUIET_ZONE;
    let mut path = String::new();
    for y in 0..code.size() {
        for x in 0..code.size() {
            if code.get_module(x, y) {
                let (left, top) = (x + QUIET_ZONE, y + QUIET_ZONE);
                write!(path, "M{left},{top}h1v1h-1z").expect("writing to a String cannot fail");
            }
        }
    }
    format!(
        "<svg class=\"qr\" xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {side} {side}\" \
         shape-rendering=\"crispEdges\" aria-hidden=\"true\">\
         <rect width=\"{side}\" height=\"{side}\" fill=\"#fff\"/>\
         <path d=\"{path}\" fill=\"#000\"/></svg>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINK: &str = "https://smoke.example.org/invite/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn the_drawing_has_one_square_per_dark_module_inside_a_quiet_zone() {
        let code = QrCode::encode_text(LINK, QrCodeEcc::Medium).unwrap();
        let dark = (0..code.size())
            .flat_map(|y| (0..code.size()).map(move |x| (x, y)))
            .filter(|&(x, y)| code.get_module(x, y))
            .count();
        let drawing = svg(LINK);
        let side = code.size() + 8;
        assert!(drawing.contains(&format!("viewBox=\"0 0 {side} {side}\"")));
        assert_eq!(drawing.matches("h1v1h-1z").count(), dark);
        assert!(
            drawing.contains("M4,4h1v1h-1z"),
            "the finder pattern starts at the margin"
        );
    }

    #[test]
    fn the_drawing_needs_no_style_and_carries_only_the_code() {
        let drawing = svg(LINK);
        assert!(!drawing.contains("style"));
        assert!(
            !drawing.contains("invite"),
            "the link is drawn, never written"
        );
    }
}
