//! Item identity. Text follows D-016, images D-015. Identity never changes stored content.

use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

pub type Identity = [u8; 32];

fn sha256(parts: &[&[u8]]) -> Identity {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// NFC (never NFKC), CRLF/CR → LF, outer newlines ignored unless the text is only newlines.
/// Everything else (spaces, tabs, case, invisible characters) stays significant.
pub fn normalize_text(text: &str) -> String {
    let nfc: String = text.nfc().collect();
    let unified = nfc.replace("\r\n", "\n").replace('\r', "\n");
    if unified.chars().all(|c| c == '\n') {
        return unified;
    }
    unified.trim_matches('\n').to_owned()
}

pub fn text_identity(text: &str) -> Identity {
    sha256(&[normalize_text(text).as_bytes()])
}

/// Hash of the decoded pixels, so the same picture from PNG, DIB or a saved file matches.
pub fn image_identity(width: u32, height: u32, rgba: &[u8]) -> Identity {
    sha256(&[&width.to_le_bytes(), &height.to_le_bytes(), rgba])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_texts_share_an_identity() {
        let same = [
            ("a\r\nb", "a\nb", "CRLF equals LF"),
            ("a\rb", "a\nb", "CR equals LF"),
            ("\n\nhello\n", "hello", "outer newlines are ignored"),
            ("\r\nhello\r\n", "hello", "outer CRLF is ignored"),
            (
                "e\u{301}",
                "\u{e9}",
                "NFC: decomposed é equals precomposed é",
            ),
            ("\r\n", "\n", "a lone CRLF is a lone newline"),
        ];
        for (a, b, why) in same {
            assert_eq!(text_identity(a), text_identity(b), "{why}");
        }
    }

    #[test]
    fn different_texts_stay_apart() {
        let different = [
            ("\n", "\n\n", "newline-only texts keep their count"),
            ("İ", "I", "Turkish dotted and dotless capitals"),
            ("ı", "i", "Turkish dotless and dotted small letters"),
            ("Value", "value", "case matters"),
            ("Value ", "Value", "trailing space matters"),
            ("\tx", "x", "indentation matters"),
            ("\u{fb01}", "fi", "no NFKC: the ligature is not fi"),
            ("a\n\nb", "a\nb", "inner blank lines matter"),
            ("a\u{200b}", "a", "zero-width space is content"),
            ("\u{feff}a", "a", "BOM is content"),
        ];
        for (a, b, why) in different {
            assert_ne!(text_identity(a), text_identity(b), "{why}");
        }
    }

    #[test]
    fn image_identity_depends_on_size_and_pixels() {
        let pixels = [255u8; 16];
        assert_eq!(image_identity(2, 2, &pixels), image_identity(2, 2, &pixels));
        assert_ne!(image_identity(2, 2, &pixels), image_identity(4, 1, &pixels));
        assert_ne!(
            image_identity(2, 2, &pixels),
            image_identity(2, 2, &[0u8; 16])
        );
    }
}
