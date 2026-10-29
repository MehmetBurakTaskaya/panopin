//! Link previews (D-057). Off until the user turns them on; then each copied link is visited once
//! from Rust, through the network guard, for its title, description, picture and icon. The
//! webview never reaches a website: pictures are re-encoded into the item's media folder and
//! served by item id.

mod guard;
mod page;

use std::{fs, io::Cursor, path::Path};

use image::{imageops::FilterType, ImageFormat, ImageReader, Limits, RgbaImage};
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE},
    Client, Url,
};
use rusqlite::{params, Connection, OptionalExtension};
use tauri::{AppHandle, Manager};
use tokio::sync::Notify;

use crate::{
    error::{AppError, Result},
    media::{thumbnails, MediaStore},
    store::search,
    AppState,
};

use guard::check_url;

/// Pictures and icons larger than this are not decoded (a small file can unpack to a huge image).
const MAX_PIXELS_SIDE: u32 = 8192;
const ICON_SIDE: u32 = 64;

const WAITING: i64 = 0;
const DONE: i64 = 1;
const FAILED: i64 = 2;
const REFUSED: i64 = 3;

#[derive(Default)]
pub struct LinkQueue(Notify);

pub fn start(app: &AppHandle) {
    app.manage(LinkQueue::default());
    let worker = app.clone();
    tauri::async_runtime::spawn(async move { run(worker).await });
    wake(app);
}

/// Looks for links to preview: after captures, a refresh or turning previews on.
pub fn wake(app: &AppHandle) {
    if let Some(queue) = app.try_state::<LinkQueue>() {
        queue.0.notify_one();
    }
}

fn next_waiting(conn: &Connection) -> Result<Option<(i64, String)>> {
    Ok(conn
        .query_row(
            "SELECT id, url FROM items
             WHERE kind = 'link' AND link_state = ?1 AND trashed_at IS NULL AND url IS NOT NULL
             ORDER BY last_copied_at DESC LIMIT 1",
            [WAITING],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?)
}

/// "Refresh preview": the link waits to be visited again.
pub fn refresh(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE items SET link_state = ?2 WHERE id = ?1 AND kind = 'link'",
        params![id, WAITING],
    )?;
    Ok(())
}

struct Fetched {
    meta: page::PageMeta,
    image: Option<thumbnails::Thumbnail>,
    icon: Option<thumbnails::Thumbnail>,
}

enum Failure {
    Refused,
    Failed,
}

async fn run(app: AppHandle) {
    let client = match guard::client() {
        Ok(client) => client,
        Err(err) => {
            log::error!("link previews are unavailable: {err}");
            return;
        }
    };
    let queue = app.state::<LinkQueue>();
    loop {
        queue.0.notified().await;
        loop {
            let state = app.state::<AppState>();
            if !state.settings().link_previews {
                break;
            }
            let Ok(Some((id, url))) = state.with_db(|db| next_waiting(db)) else {
                break;
            };
            let outcome = fetch(&client, &url).await;
            let Some(media) = crate::capture::media(&app) else {
                break;
            };
            let stored =
                state.with_db(|db| store(db, &media, id, outcome, crate::capture::now_ms()));
            match stored {
                // Only ids and outcomes are logged; the address itself is copied content.
                Ok(state) => log::info!("link preview of item {id}: state {state}"),
                Err(err) => {
                    log::error!("link preview of item {id} could not be stored: {err}");
                    break;
                }
            }
            crate::capture::changed(&app);
        }
    }
}

async fn fetch(client: &Client, raw: &str) -> std::result::Result<Fetched, Failure> {
    let url = Url::parse(raw.trim()).map_err(|_| Failure::Refused)?;
    check_url(&url).map_err(|_| Failure::Refused)?;
    let meta = if page::is_youtube_video(&url) {
        let oembed = page::oembed_url(&url).ok_or(Failure::Refused)?;
        let body = get(client, &oembed, "application/json", true).await?;
        page::parse_oembed(&body.0).ok_or(Failure::Failed)?
    } else {
        let (body, final_url, content_type) = get(
            client,
            &url,
            "text/html,application/xhtml+xml;q=0.9,*/*;q=0.1",
            true,
        )
        .await?;
        let html = content_type.as_deref().is_some_and(|t| t.contains("html"));
        if html {
            page::parse(&page::decode(&body, content_type.as_deref()), &final_url)
        } else {
            page::PageMeta::default()
        }
    };
    let image = match &meta.image {
        Some(image) => picture(client, image, false).await,
        None => None,
    };
    let icon_url = meta.icon.clone().or_else(|| url.join("/favicon.ico").ok());
    let icon = match &icon_url {
        Some(icon) => picture(client, icon, true).await,
        None => None,
    };
    Ok(Fetched { meta, image, icon })
}

/// One guarded GET: the body (within the limits), the final URL after redirects and the type.
async fn get(
    client: &Client,
    url: &Url,
    accept: &str,
    truncate: bool,
) -> std::result::Result<(Vec<u8>, Url, Option<String>), Failure> {
    check_url(url).map_err(|_| Failure::Refused)?;
    let response = client
        .get(url.clone())
        .header(ACCEPT, accept)
        .send()
        .await
        .map_err(|_| Failure::Failed)?;
    if !response.status().is_success() {
        return Err(Failure::Failed);
    }
    let final_url = response.url().clone();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_ascii_lowercase);
    let body = guard::read_body(response, truncate)
        .await
        .map_err(|_| Failure::Failed)?;
    Ok((body, final_url, content_type))
}

/// A picture or icon, decoded with limits and re-encoded, so no remote bytes are ever served.
async fn picture(client: &Client, url: &Url, icon: bool) -> Option<thumbnails::Thumbnail> {
    let (body, _, _) = get(client, url, "image/*", false).await.ok()?;
    let pixels = decode_limited(&body)?;
    if icon {
        let small = if pixels.width() > ICON_SIDE || pixels.height() > ICON_SIDE {
            image::imageops::resize(&pixels, ICON_SIDE, ICON_SIDE, FilterType::Triangle)
        } else {
            pixels
        };
        let mut bytes = Vec::new();
        small
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .ok()?;
        Some(thumbnails::Thumbnail { bytes, ext: "png" })
    } else {
        thumbnails::make(&pixels).ok()
    }
}

fn decode_limited(bytes: &[u8]) -> Option<RgbaImage> {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_PIXELS_SIDE);
    limits.max_image_height = Some(MAX_PIXELS_SIDE);
    limits.max_alloc = Some(256 * 1024 * 1024);
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    reader.limits(limits);
    Some(reader.decode().ok()?.into_rgba8())
}

fn write_atomically(dir: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!("{name}.tmp"));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, dir.join(name))?;
    Ok(())
}

/// Writes the preview and returns the state it left the link in.
fn store(
    conn: &Connection,
    media: &MediaStore,
    id: i64,
    outcome: std::result::Result<Fetched, Failure>,
    now: i64,
) -> Result<i64> {
    let fetched = match outcome {
        Ok(fetched) => fetched,
        Err(failure) => {
            let state = match failure {
                Failure::Refused => REFUSED,
                Failure::Failed => FAILED,
            };
            conn.execute(
                "UPDATE items SET link_state = ?2, link_checked_at = ?3 WHERE id = ?1",
                params![id, state, now],
            )?;
            return Ok(state);
        }
    };
    let dir = media.dir(id);
    // Old files with another extension are removed when the new ones are written.
    let save =
        |name: &str, picture: &Option<thumbnails::Thumbnail>| -> Result<Option<&'static str>> {
            for ext in ["jpg", "png"] {
                let _ = fs::remove_file(dir.join(format!("{name}.{ext}")));
            }
            match picture {
                Some(picture) => {
                    write_atomically(&dir, &format!("{name}.{}", picture.ext), &picture.bytes)?;
                    Ok(Some(picture.ext))
                }
                None => Ok(None),
            }
        };
    let image_ext = save("link-image", &fetched.image)?;
    let icon_ext = save("link-icon", &fetched.icon)?;
    let meta = &fetched.meta;
    let changed = conn.execute(
        "UPDATE items SET link_state = ?2, link_title = ?3, link_description = ?4, link_site = ?5,
                          link_image_ext = ?6, link_icon_ext = ?7, link_checked_at = ?8
         WHERE id = ?1",
        params![
            id,
            DONE,
            meta.title,
            meta.description,
            meta.site,
            image_ext,
            icon_ext,
            now
        ],
    )?;
    if changed == 1 {
        search::index(conn, id)?;
    }
    Ok(DONE)
}

/// Ctrl+O: opens a link item in the default browser. Only http and https leave Panopin.
pub fn open(conn: &Connection, id: i64) -> Result<()> {
    let url: Option<String> = conn
        .query_row(
            "SELECT url FROM items WHERE id = ?1 AND kind = 'link'",
            [id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let url = url
        .and_then(|u| Url::parse(u.trim()).ok())
        .filter(|u| matches!(u.scheme(), "http" | "https"))
        .ok_or(AppError::NotEditable)?;
    open_in_browser(url.as_str())
}

/// Opens one of Panopin's own pages (Settings → About).
pub fn open_url(url: &str) -> Result<()> {
    open_in_browser(url)
}

#[cfg(windows)]
fn open_in_browser(url: &str) -> Result<()> {
    use windows::{
        core::{w, HSTRING},
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    };
    // SAFETY: plain strings; ShellExecute hands an http(s) URL to the default browser.
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &HSTRING::from(url),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    // Values above 32 mean success.
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(windows::core::Error::from_thread().into())
    }
}

#[cfg(not(windows))]
fn open_in_browser(url: &str) -> Result<()> {
    std::process::Command::new("open").arg(url).spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store;

    fn link(conn: &Connection, url: &str, copied: i64) -> i64 {
        conn.execute(
            "INSERT INTO items (kind, identity, created_at, last_copied_at, source, url, text_body)
             VALUES ('link', randomblob(32), ?2, ?2, 'clipboard', ?1, ?1)",
            params![url, copied],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn previews_are_stored_and_searchable_and_failures_wait_for_a_refresh() {
        let conn = store::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let media = MediaStore::new(dir.path().to_path_buf());
        let old = link(&conn, "https://example.com/a", 1);
        let new = link(&conn, "https://example.org/b", 2);
        assert_eq!(next_waiting(&conn).unwrap().map(|j| j.0), Some(new));
        let fetched = Fetched {
            meta: page::PageMeta {
                title: Some("Kış Bahçesi".into()),
                site: Some("Örnek".into()),
                ..page::PageMeta::default()
            },
            image: Some(thumbnails::Thumbnail {
                bytes: vec![1, 2, 3],
                ext: "jpg",
            }),
            icon: None,
        };
        assert_eq!(store(&conn, &media, new, Ok(fetched), 9).unwrap(), DONE);
        assert!(media.dir(new).join("link-image.jpg").exists());
        assert_eq!(search::find(&conn, "kış bahç", false).unwrap(), vec![new]);
        assert_eq!(next_waiting(&conn).unwrap().map(|j| j.0), Some(old));
        assert_eq!(
            store(&conn, &media, old, Err(Failure::Refused), 9).unwrap(),
            REFUSED
        );
        assert_eq!(next_waiting(&conn).unwrap(), None);
        refresh(&conn, old).unwrap();
        assert_eq!(next_waiting(&conn).unwrap().map(|j| j.0), Some(old));
    }

    /// Visits real public pages through the guard; needs the internet, so it runs on request only:
    /// `cargo test -- --ignored live_previews`.
    #[test]
    #[ignore]
    fn live_previews() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let client = guard::client().unwrap();
            for url in [
                "https://example.com/",
                "https://github.com/tauri-apps/tauri",
                "https://www.youtube.com/watch?v=jNQXAC9IVRw",
            ] {
                match fetch(&client, url).await {
                    Ok(f) => println!(
                        "{url}: title {:?}, site {:?}, picture {}, icon {}",
                        f.meta.title,
                        f.meta.site,
                        f.image.is_some(),
                        f.icon.is_some()
                    ),
                    Err(Failure::Refused) => println!("{url}: refused"),
                    Err(Failure::Failed) => println!("{url}: failed"),
                }
            }
        });
    }

    #[test]
    fn remote_pictures_are_decoded_within_limits() {
        let mut png = Vec::new();
        RgbaImage::new(4, 3)
            .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .unwrap();
        assert_eq!(decode_limited(&png).unwrap().dimensions(), (4, 3));
        assert!(decode_limited(b"<svg xmlns='http://www.w3.org/2000/svg'/>").is_none());
        // A PNG header claiming 100 000 × 100 000 pixels is refused before decoding.
        let mut huge = png.clone();
        huge[16..20].copy_from_slice(&100_000u32.to_be_bytes());
        huge[20..24].copy_from_slice(&100_000u32.to_be_bytes());
        assert!(decode_limited(&huge).is_none());
    }
}
