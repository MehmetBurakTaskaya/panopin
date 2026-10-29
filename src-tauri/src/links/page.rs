//! What a page says about itself: title, description, picture and icon from its `<head>`. The
//! HTML is only tokenised, never run (html5gum); YouTube links ask oEmbed instead.

use html5gum::{DefaultEmitter, Token, Tokenizer};
use reqwest::Url;
use serde::Deserialize;

const MAX_TITLE: usize = 300;
const MAX_DESCRIPTION: usize = 500;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PageMeta {
    pub title: Option<String>,
    pub description: Option<String>,
    pub site: Option<String>,
    pub image: Option<Url>,
    pub icon: Option<Url>,
}

/// Collapses whitespace and cuts long texts at a character boundary.
fn clean(text: &str, max: usize) -> Option<String> {
    let joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if joined.is_empty() {
        return None;
    }
    let mut out: String = joined.chars().take(max).collect();
    if joined.chars().count() > max {
        out.push('…');
    }
    Some(out)
}

/// The character set from `Content-Type`, else from a `<meta>` near the top, else UTF-8.
pub fn decode(bytes: &[u8], content_type: Option<&str>) -> String {
    let label = content_type
        .and_then(charset_of)
        .or_else(|| sniff_meta_charset(&bytes[..bytes.len().min(2048)]));
    let encoding = label
        .and_then(|label| encoding_rs::Encoding::for_label(label.as_bytes()))
        .unwrap_or(encoding_rs::UTF_8);
    encoding.decode(bytes).0.into_owned()
}

fn charset_of(content_type: &str) -> Option<String> {
    content_type.split(';').find_map(|part| {
        let (key, value) = part.split_once('=')?;
        (key.trim().eq_ignore_ascii_case("charset"))
            .then(|| value.trim().trim_matches('"').to_owned())
    })
}

fn sniff_meta_charset(head: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(head).to_ascii_lowercase();
    let at = text.find("charset=")? + "charset=".len();
    let rest = text[at..].trim_start_matches(['"', '\'']);
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .unwrap_or(rest.len());
    (end > 0).then(|| rest[..end].to_owned())
}

fn attr(tag: &html5gum::StartTag<()>, name: &str) -> Option<String> {
    tag.attributes
        .get(name.as_bytes())
        .map(|value| String::from_utf8_lossy(value).into_owned())
}

/// Reads the `<head>`: Open Graph and Twitter tags first, then `<title>` and the description.
pub fn parse(html: &str, page: &Url) -> PageMeta {
    let mut emitter = DefaultEmitter::default();
    // <script>, <style> and <title> contents are text, as browsers treat them.
    emitter.naively_switch_states(true);
    let mut base = page.clone();
    let (mut og_title, mut tw_title, mut title) = (None, None, None);
    let (mut og_desc, mut tw_desc, mut desc) = (None, None, None);
    let (mut og_image, mut tw_image, mut site) = (None, None, None);
    let (mut icon, mut touch_icon) = (None, None);
    let mut in_title = false;
    let mut title_text = String::new();
    for token in Tokenizer::new_with_emitter(html, emitter).flatten() {
        match token {
            Token::StartTag(tag) => {
                let name = String::from_utf8_lossy(&tag.name).to_ascii_lowercase();
                match name.as_str() {
                    "body" => break,
                    "title" => in_title = title.is_none(),
                    "base" => {
                        if let Some(href) = attr(&tag, "href").and_then(|h| page.join(&h).ok()) {
                            base = href;
                        }
                    }
                    "meta" => {
                        let key = attr(&tag, "property")
                            .or_else(|| attr(&tag, "name"))
                            .map(|k| k.to_ascii_lowercase());
                        let content = attr(&tag, "content");
                        match (key.as_deref(), content) {
                            (Some("og:title"), Some(c)) => og_title = og_title.or(Some(c)),
                            (Some("twitter:title"), Some(c)) => tw_title = tw_title.or(Some(c)),
                            (Some("og:description"), Some(c)) => og_desc = og_desc.or(Some(c)),
                            (Some("twitter:description"), Some(c)) => tw_desc = tw_desc.or(Some(c)),
                            (Some("description"), Some(c)) => desc = desc.or(Some(c)),
                            (
                                Some("og:image" | "og:image:secure_url" | "og:image:url"),
                                Some(c),
                            ) => og_image = og_image.or(Some(c)),
                            (Some("twitter:image" | "twitter:image:src"), Some(c)) => {
                                tw_image = tw_image.or(Some(c))
                            }
                            (Some("og:site_name"), Some(c)) => site = site.or(Some(c)),
                            _ => {}
                        }
                    }
                    "link" => {
                        let rel = attr(&tag, "rel").unwrap_or_default().to_ascii_lowercase();
                        let href = attr(&tag, "href");
                        let rels: Vec<&str> = rel.split_whitespace().collect();
                        if rels.contains(&"icon") && icon.is_none() {
                            icon = href;
                        } else if rels.iter().any(|r| r.starts_with("apple-touch-icon")) {
                            touch_icon = touch_icon.or(href);
                        }
                    }
                    _ => {}
                }
            }
            Token::String(text) if in_title => title_text.push_str(&String::from_utf8_lossy(&text)),
            Token::EndTag(tag) => {
                let name = String::from_utf8_lossy(&tag.name).to_ascii_lowercase();
                if name == "title" && in_title {
                    in_title = false;
                    title = Some(std::mem::take(&mut title_text));
                } else if name == "head" {
                    break;
                }
            }
            _ => {}
        }
    }
    let resolve = |href: Option<String>| href.and_then(|h| base.join(h.trim()).ok());
    PageMeta {
        title: og_title
            .or(tw_title)
            .or(title)
            .and_then(|t| clean(&t, MAX_TITLE)),
        description: og_desc
            .or(tw_desc)
            .or(desc)
            .and_then(|d| clean(&d, MAX_DESCRIPTION)),
        site: site.and_then(|s| clean(&s, 120)),
        image: resolve(og_image.or(tw_image)),
        icon: resolve(icon.or(touch_icon)),
    }
}

/// A YouTube video page, whose details come from oEmbed instead of the page.
pub fn is_youtube_video(url: &Url) -> bool {
    let host = url.host_str().unwrap_or("").trim_start_matches("www.");
    match host {
        "youtube.com" | "m.youtube.com" | "music.youtube.com" => {
            (url.path() == "/watch" && url.query_pairs().any(|(k, _)| k == "v"))
                || url.path().starts_with("/shorts/")
                || url.path().starts_with("/live/")
        }
        "youtu.be" => url.path().len() > 1,
        _ => false,
    }
}

pub fn oembed_url(video: &Url) -> Option<Url> {
    Url::parse_with_params(
        "https://www.youtube.com/oembed",
        &[("format", "json"), ("url", video.as_str())],
    )
    .ok()
}

#[derive(Debug, Deserialize)]
struct OEmbed {
    title: Option<String>,
    author_name: Option<String>,
    thumbnail_url: Option<String>,
}

pub fn parse_oembed(json: &[u8]) -> Option<PageMeta> {
    let data: OEmbed = serde_json::from_slice(json).ok()?;
    Some(PageMeta {
        title: data.title.and_then(|t| clean(&t, MAX_TITLE)),
        description: None,
        site: data.author_name.and_then(|a| clean(&a, 120)),
        image: data.thumbnail_url.and_then(|u| Url::parse(&u).ok()),
        icon: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> Url {
        Url::parse("https://example.com/blog/post").unwrap()
    }

    #[test]
    fn open_graph_wins_over_the_title_and_paths_resolve() {
        let html = r#"<!doctype html><html><head>
            <meta charset="utf-8"><title>  Plain
            title </title>
            <meta property="og:title" content="Kahve &amp; Çay">
            <meta name="description" content="Bir   açıklama">
            <meta property="og:image" content="/img/cover.jpg">
            <link rel="shortcut icon" href="favicon.png">
            <script>var x = "<meta property='og:title' content='fake'>";</script>
            </head><body><meta property="og:description" content="too late"></body></html>"#;
        let meta = parse(html, &page());
        assert_eq!(meta.title.as_deref(), Some("Kahve & Çay"));
        assert_eq!(meta.description.as_deref(), Some("Bir açıklama"));
        assert_eq!(
            meta.image.unwrap().as_str(),
            "https://example.com/img/cover.jpg"
        );
        assert_eq!(
            meta.icon.unwrap().as_str(),
            "https://example.com/blog/favicon.png"
        );
    }

    #[test]
    fn the_title_tag_is_the_fallback_and_long_text_is_cut() {
        let long = "ş".repeat(400);
        let html = format!("<title>{long}</title>");
        let title = parse(&html, &page()).title.unwrap();
        assert_eq!(title.chars().count(), MAX_TITLE + 1);
        assert!(title.ends_with('…'));
        assert_eq!(parse("<p>no head</p>", &page()), PageMeta::default());
    }

    #[test]
    fn character_sets_come_from_the_header_or_the_page() {
        // "Işık" in windows-1254, the Turkish code page.
        let bytes = b"<title>I\xfe\xfdk</title>";
        assert_eq!(
            decode(bytes, Some("text/html; charset=windows-1254")),
            "<title>Işık</title>"
        );
        let with_meta = b"<meta charset=\"windows-1254\"><title>I\xfe\xfdk</title>";
        assert!(decode(with_meta, Some("text/html")).ends_with("Işık</title>"));
        assert_eq!(decode("Çay".as_bytes(), None), "Çay");
    }

    #[test]
    fn youtube_videos_use_oembed() {
        let yes = [
            "https://www.youtube.com/watch?v=abc",
            "https://youtu.be/abc",
            "https://m.youtube.com/shorts/abc",
        ];
        for url in yes {
            assert!(is_youtube_video(&Url::parse(url).unwrap()), "{url}");
        }
        assert!(!is_youtube_video(
            &Url::parse("https://www.youtube.com/@channel").unwrap()
        ));
        let oembed = oembed_url(&Url::parse("https://youtu.be/abc?t=5").unwrap()).unwrap();
        assert_eq!(
            oembed.as_str(),
            "https://www.youtube.com/oembed?format=json&url=https%3A%2F%2Fyoutu.be%2Fabc%3Ft%3D5"
        );
        let meta = parse_oembed(br#"{"title":"Bir video","author_name":"Kanal","thumbnail_url":"https://i.ytimg.com/vi/abc/hqdefault.jpg"}"#).unwrap();
        assert_eq!(meta.title.as_deref(), Some("Bir video"));
        assert_eq!(meta.site.as_deref(), Some("Kanal"));
        assert!(meta.image.is_some());
        assert!(parse_oembed(b"not json").is_none());
    }
}
