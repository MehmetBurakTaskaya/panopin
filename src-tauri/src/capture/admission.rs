//! Decides what a clipboard change becomes. Pure, so every rule is unit-tested; the platform
//! listeners only fill in a `Snapshot`.

// Only the Windows listener builds snapshots so far; the macOS one arrives in Phase 8.
#![cfg_attr(not(windows), allow(dead_code))]

use crate::settings::Sources;

/// What the listener saw on the clipboard. Content is only read when no privacy marker is set.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub owner_exe: Option<String>,
    pub foreground_exe: Option<String>,
    /// `Clipboard Viewer Ignore`, `ExcludeClipboardContentFromMonitorProcessing`, or
    /// `CanIncludeInClipboardHistory` / `CanUploadToCloudClipboard` set to 0.
    pub private: bool,
    /// Panopin's own marker format: the copy came from Panopin itself.
    pub own_copy: bool,
    /// Explorer file copies (`CF_HDROP`), not captured in v1.
    pub has_files: bool,
    pub text: Option<String>,
    pub image: Option<RawImage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawImage {
    Png(Vec<u8>),
    Dib(Vec<u8>),
}

pub struct Policy<'a> {
    pub onboarded: bool,
    pub paused: bool,
    pub sources: &'a Sources,
    pub excluded_apps: &'a [String],
}

/// Why a change was not captured. Logged without any content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skip {
    NotOnboarded,
    Paused,
    OwnCopy,
    Private,
    ExcludedApp,
    Files,
    SourceOff,
    Nothing,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Skip(Skip),
    Text(String),
    Link { text: String, url: String },
    Image(RawImage),
}

pub fn decide(snapshot: Snapshot, policy: &Policy<'_>) -> Decision {
    let excluded = |exe: &Option<String>| {
        exe.as_deref().is_some_and(|exe| {
            policy
                .excluded_apps
                .iter()
                .any(|app| app.eq_ignore_ascii_case(exe))
        })
    };
    let skip = if !policy.onboarded {
        Some(Skip::NotOnboarded)
    } else if policy.paused {
        Some(Skip::Paused)
    } else if snapshot.own_copy {
        Some(Skip::OwnCopy)
    } else if snapshot.private {
        Some(Skip::Private)
    } else if excluded(&snapshot.owner_exe) || excluded(&snapshot.foreground_exe) {
        Some(Skip::ExcludedApp)
    } else if snapshot.has_files {
        Some(Skip::Files)
    } else {
        None
    };
    if let Some(reason) = skip {
        return Decision::Skip(reason);
    }

    // A copy that carries real text is a text copy (an app may add a rendered picture of it).
    // Whitespace-only text does not count; accepted text is stored exactly as copied.
    if let Some(text) = snapshot
        .text
        .filter(|t| !t.chars().all(char::is_whitespace))
    {
        return match link_url(&text).map(str::to_owned) {
            Some(url) if policy.sources.links => Decision::Link { text, url },
            Some(_) => Decision::Skip(Skip::SourceOff),
            None if policy.sources.clipboard_text => Decision::Text(text),
            None => Decision::Skip(Skip::SourceOff),
        };
    }
    match snapshot.image {
        Some(image) if policy.sources.clipboard_images => Decision::Image(image),
        Some(_) => Decision::Skip(Skip::SourceOff),
        None => Decision::Skip(Skip::Nothing),
    }
}

/// The URL when the whole text (outer whitespace aside) is a single http(s) URL.
pub fn link_url(text: &str) -> Option<&str> {
    let url = text.trim();
    let starts = |prefix: &str| {
        url.get(..prefix.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(prefix))
    };
    if !(starts("https://") || starts("http://")) || url.chars().any(char::is_whitespace) {
        return None;
    }
    let parsed = url::Url::parse(url).ok()?;
    let has_host = parsed.host_str().is_some_and(|host| !host.is_empty());
    (matches!(parsed.scheme(), "http" | "https") && has_host).then_some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(t: &str) -> Snapshot {
        Snapshot {
            text: Some(t.to_owned()),
            ..Snapshot::default()
        }
    }

    fn decide_with(
        snapshot: Snapshot,
        edit: impl FnOnce(&mut Sources, &mut Vec<String>),
    ) -> Decision {
        let mut sources = Sources::default();
        let mut excluded = vec!["KeePassXC.exe".to_owned()];
        edit(&mut sources, &mut excluded);
        decide(
            snapshot,
            &Policy {
                onboarded: true,
                paused: false,
                sources: &sources,
                excluded_apps: &excluded,
            },
        )
    }

    fn decide_default(snapshot: Snapshot) -> Decision {
        decide_with(snapshot, |_, _| {})
    }

    #[test]
    fn whole_text_urls_become_links() {
        for (input, url) in [
            ("https://example.com", "https://example.com"),
            (
                "  https://example.com/a?b=1#c \r\n",
                "https://example.com/a?b=1#c",
            ),
            ("HTTP://EXAMPLE.COM", "HTTP://EXAMPLE.COM"),
            ("http://localhost:3000/x", "http://localhost:3000/x"),
        ] {
            assert_eq!(link_url(input), Some(url), "{input:?}");
        }
        for input in [
            "see https://example.com",
            "ftp://example.com",
            "https://",
            "https://exa mple.com",
            "example.com",
            "https:/x",
        ] {
            assert_eq!(link_url(input), None, "{input:?}");
        }
    }

    #[test]
    fn text_is_kept_exactly() {
        let body = "  indented\r\nline\u{200b}\n";
        assert_eq!(decide_default(text(body)), Decision::Text(body.to_owned()));
        assert_eq!(
            decide_default(text(" https://example.com ")),
            Decision::Link {
                text: " https://example.com ".into(),
                url: "https://example.com".into()
            }
        );
    }

    #[test]
    fn whitespace_only_text_is_ignored() {
        for blank in ["", " ", "\t\r\n", "\u{a0}\u{3000}\u{2028}"] {
            assert_eq!(
                decide_default(text(blank)),
                Decision::Skip(Skip::Nothing),
                "{blank:?}"
            );
        }
        // zero-width space and BOM are content
        assert!(matches!(
            decide_default(text("\u{200b}")),
            Decision::Text(_)
        ));
        assert!(matches!(
            decide_default(text("\u{feff}")),
            Decision::Text(_)
        ));
    }

    #[test]
    fn privacy_and_state_rules_win() {
        let private = Snapshot {
            private: true,
            ..text("secret")
        };
        assert_eq!(decide_default(private), Decision::Skip(Skip::Private));
        let own = Snapshot {
            own_copy: true,
            ..text("mine")
        };
        assert_eq!(decide_default(own), Decision::Skip(Skip::OwnCopy));
        let files = Snapshot {
            has_files: true,
            ..text("C:\\a.txt")
        };
        assert_eq!(decide_default(files), Decision::Skip(Skip::Files));
        let sources = Sources::default();
        let excluded: Vec<String> = Vec::new();
        let paused = Policy {
            onboarded: true,
            paused: true,
            sources: &sources,
            excluded_apps: &excluded,
        };
        assert_eq!(decide(text("x"), &paused), Decision::Skip(Skip::Paused));
        let fresh = Policy {
            onboarded: false,
            paused: false,
            sources: &sources,
            excluded_apps: &excluded,
        };
        assert_eq!(
            decide(text("x"), &fresh),
            Decision::Skip(Skip::NotOnboarded)
        );
    }

    #[test]
    fn excluded_apps_match_owner_or_foreground_case_insensitively() {
        let owner = Snapshot {
            owner_exe: Some("keepassxc.EXE".into()),
            ..text("pw")
        };
        assert_eq!(decide_default(owner), Decision::Skip(Skip::ExcludedApp));
        let foreground = Snapshot {
            foreground_exe: Some("KeePassXC.exe".into()),
            ..text("pw")
        };
        assert_eq!(
            decide_default(foreground),
            Decision::Skip(Skip::ExcludedApp)
        );
        let other = Snapshot {
            owner_exe: Some("notepad.exe".into()),
            ..text("ok")
        };
        assert_eq!(decide_default(other), Decision::Text("ok".into()));
    }

    #[test]
    fn sources_switch_independently() {
        let off = |s: &mut Sources, _: &mut Vec<String>| {
            s.clipboard_text = false;
            s.links = false;
        };
        assert_eq!(
            decide_with(text("note"), off),
            Decision::Skip(Skip::SourceOff)
        );
        assert_eq!(
            decide_with(text("https://a.example"), off),
            Decision::Skip(Skip::SourceOff)
        );
        let only_text_off = |s: &mut Sources, _: &mut Vec<String>| s.clipboard_text = false;
        assert!(matches!(
            decide_with(text("https://a.example"), only_text_off),
            Decision::Link { .. }
        ));
        let png = Snapshot {
            image: Some(RawImage::Png(vec![1])),
            ..Snapshot::default()
        };
        assert_eq!(
            decide_default(png.clone()),
            Decision::Image(RawImage::Png(vec![1]))
        );
        let images_off = |s: &mut Sources, _: &mut Vec<String>| s.clipboard_images = false;
        assert_eq!(
            decide_with(png, images_off),
            Decision::Skip(Skip::SourceOff)
        );
    }

    #[test]
    fn text_wins_over_a_rendered_picture() {
        let both = Snapshot {
            image: Some(RawImage::Dib(vec![0])),
            ..text("A1")
        };
        assert_eq!(decide_default(both), Decision::Text("A1".into()));
        let blank_text = Snapshot {
            image: Some(RawImage::Dib(vec![0])),
            ..text("  ")
        };
        assert_eq!(
            decide_default(blank_text),
            Decision::Image(RawImage::Dib(vec![0]))
        );
    }
}
