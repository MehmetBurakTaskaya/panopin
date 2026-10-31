//! Paste one by one on a Mac (D-084). The current item goes on the pasteboard with its data
//! promised by a provider: macOS asks the provider only when an app pastes, and shortly after,
//! the next item takes its place. AppKit calls the provider on the main thread, where the queue
//! does its pasteboard work too.

use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicIsize, AtomicU64, Ordering},
        mpsc, Mutex, MutexGuard,
    },
    thread,
    time::Instant,
};

use objc2::{
    define_class, msg_send,
    rc::{autoreleasepool, Retained},
    runtime::{NSObject, NSObjectProtocol, ProtocolObject},
    AnyThread, DefinedClass,
};
use objc2_app_kit::{
    NSPasteboard, NSPasteboardContentsOptions, NSPasteboardItem, NSPasteboardItemDataProvider,
    NSPasteboardType, NSPasteboardTypeFileURL, NSPasteboardTypePNG, NSPasteboardTypeString,
    NSPasteboardWriting,
};
use objc2_foundation::{NSArray, NSData, NSString, NSURL};
use tauri::AppHandle;

use super::{
    clipboard_mac,
    queue::{self, QueueStatus, Turns, SETTLE},
    Formats,
};
use crate::{
    capture::clipboard_mac::OWN_TYPE,
    error::{AppError, Result},
};

/// Tells clipboard tools that follow nspasteboard.org not to keep the item.
const TRANSIENT: &str = "org.nspasteboard.TransientType";

static TURNS: Mutex<Option<Turns>> = Mutex::new(None);
/// Counts the items put on the pasteboard, so a provider or a timer of an older one does nothing.
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// The pasteboard's change count right after the current item went on it.
static OFFERED_AT: AtomicIsize = AtomicIsize::new(0);
/// When an app last took the current item.
static TAKEN_AT: Mutex<Option<Instant>> = Mutex::new(None);

thread_local! {
    /// The item on the pasteboard and its provider, kept alive while they are there.
    static OFFERED: RefCell<Option<(Retained<NSPasteboardItem>, Retained<Provider>)>> =
        const { RefCell::new(None) };
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct Ivars {
    generation: u64,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "PanopinPasteQueueProvider"]
    #[ivars = Ivars]
    struct Provider;

    unsafe impl NSObjectProtocol for Provider {}

    unsafe impl NSPasteboardItemDataProvider for Provider {
        #[unsafe(method(pasteboard:item:provideDataForType:))]
        fn provide_data(
            &self,
            _pasteboard: Option<&NSPasteboard>,
            item: &NSPasteboardItem,
            kind: &NSPasteboardType,
        ) {
            provide(self.ivars().generation, item, kind);
        }

        #[unsafe(method(pasteboardFinishedWithDataProvider:))]
        fn finished_with(&self, _pasteboard: &NSPasteboard) {
            finished(self.ivars().generation);
        }
    }
);

impl Provider {
    fn new(generation: u64) -> Retained<Self> {
        let this = Self::alloc().set_ivars(Ivars { generation });
        // SAFETY: NSObject's plain initialiser.
        unsafe { msg_send![super(this), init] }
    }
}

/// Puts the first item on the pasteboard; the others follow one paste at a time.
pub fn start(app: &AppHandle, items: Vec<Formats>) -> Result<()> {
    queue::attach(app);
    let Some(turns) = Turns::new(items) else {
        return Ok(());
    };
    let (tx, rx) = mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(begin(turns));
    })?;
    rx.recv().map_err(|_| AppError::Interrupted)?
}

/// Panopin quits: the item waiting goes on the pasteboard in full, so the pasteboard keeps it.
pub fn finish() {
    let Some(turns) = lock(&TURNS).take() else {
        return;
    };
    // The provider hears that the pasteboard moved on; that is this write, not someone else's.
    GENERATION.fetch_add(1, Ordering::SeqCst);
    if let Err(err) = clipboard_mac::write(turns.current()) {
        log::warn!("the queued item could not stay on the pasteboard: {err}");
    }
}

fn begin(turns: Turns) -> Result<()> {
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    offer(turns.current(), generation)?;
    let status = turns.status();
    *lock(&TURNS) = Some(turns);
    announce(status);
    Ok(())
}

/// Puts the item on the pasteboard with its data promised, marked as Panopin's own and as not for
/// clipboard tools or other devices.
fn offer(formats: &Formats, generation: u64) -> Result<()> {
    autoreleasepool(|_| {
        // SAFETY: the pasteboard type constants are immutable statics provided by AppKit.
        let (string, png, file_url) = unsafe {
            (
                NSPasteboardTypeString,
                NSPasteboardTypePNG,
                NSPasteboardTypeFileURL,
            )
        };
        let mut promised: Vec<&NSPasteboardType> = Vec::new();
        if formats.text.is_some() {
            promised.push(string);
        }
        if formats.png.is_some() {
            promised.push(png);
        }
        if !formats.paths.is_empty() {
            promised.push(file_url);
        }
        let provider = Provider::new(generation);
        let item = NSPasteboardItem::new();
        if !item.setDataProvider_forTypes(
            ProtocolObject::from_ref(&*provider),
            &NSArray::from_slice(&promised),
        ) {
            return Err(AppError::Mac(
                "the queued item could not be prepared".into(),
            ));
        }
        item.setData_forType(&NSData::with_bytes(&[1]), &NSString::from_str(OWN_TYPE));
        item.setData_forType(&NSData::with_bytes(&[]), &NSString::from_str(TRANSIENT));
        let pasteboard = NSPasteboard::generalPasteboard();
        // Universal Clipboard would fetch the data at once, as if pasted on another device.
        pasteboard.prepareForNewContentsWithOptions(NSPasteboardContentsOptions::CurrentHostOnly);
        let objects = [ProtocolObject::<dyn NSPasteboardWriting>::from_retained(
            item.clone(),
        )];
        if !pasteboard.writeObjects(&NSArray::from_retained_slice(&objects)) {
            return Err(AppError::Mac(
                "the queued item could not be put on the pasteboard".into(),
            ));
        }
        OFFERED_AT.store(pasteboard.changeCount(), Ordering::SeqCst);
        *lock(&TAKEN_AT) = None;
        OFFERED.with(|slot| *slot.borrow_mut() = Some((item, provider)));
        Ok(())
    })
}

/// An app pastes: it gets the current item's data of this type, and the next item follows once
/// the paste has settled.
fn provide(generation: u64, item: &NSPasteboardItem, kind: &NSPasteboardType) {
    if generation != GENERATION.load(Ordering::SeqCst) {
        return;
    }
    let running = {
        let mut slot = lock(&TURNS);
        let Some(turns) = slot.as_mut() else {
            return;
        };
        let formats = turns.current();
        // SAFETY: as in `offer`.
        let (string, png, file_url) = unsafe {
            (
                NSPasteboardTypeString,
                NSPasteboardTypePNG,
                NSPasteboardTypeFileURL,
            )
        };
        if kind == string {
            if let Some(text) = &formats.text {
                let units = text.strip_suffix(&[0]).unwrap_or(text);
                item.setString_forType(
                    &NSString::from_str(&String::from_utf16_lossy(units)),
                    string,
                );
            }
        } else if kind == png {
            if let Some(bytes) = &formats.png {
                item.setData_forType(&NSData::with_bytes(bytes), png);
            }
        } else if kind == file_url {
            if let Some(path) = formats.paths.first() {
                let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
                if let Some(address) = url.absoluteString() {
                    item.setString_forType(&address, file_url);
                }
            }
        }
        turns.take();
        turns.running()
    };
    if running {
        *lock(&TAKEN_AT) = Some(Instant::now());
        settle_then_advance(generation);
    }
}

fn settle_then_advance(generation: u64) {
    let Some(app) = queue::app().cloned() else {
        return;
    };
    let spawned = thread::Builder::new()
        .name("paste-queue".into())
        .spawn(move || {
            thread::sleep(SETTLE);
            let _ = app.run_on_main_thread(move || advance(generation));
        });
    if let Err(err) = spawned {
        log::warn!("the queue could not wait for the paste to settle: {err}");
    }
}

fn advance(generation: u64) {
    if generation != GENERATION.load(Ordering::SeqCst) {
        return;
    }
    // A later read of the same paste waits for its own turn.
    if lock(&TAKEN_AT).is_some_and(|at| at.elapsed() < SETTLE) {
        return;
    }
    let status = {
        let mut slot = lock(&TURNS);
        let Some(turns) = slot.as_mut() else {
            return;
        };
        if turns.advance() {
            let next = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
            if let Err(err) = offer(turns.current(), next) {
                log::warn!("the next queued item could not go on the pasteboard: {err}");
                *slot = None;
                None
            } else {
                turns.status()
            }
        } else {
            turns.status()
        }
    };
    announce(status);
}

/// The pasteboard is done with a provider: either another copy took its place, which ends the
/// queue, or it has all the data it wanted.
fn finished(generation: u64) {
    if generation != GENERATION.load(Ordering::SeqCst) {
        return;
    }
    let count = NSPasteboard::generalPasteboard().changeCount();
    if count == OFFERED_AT.load(Ordering::SeqCst) {
        return;
    }
    if lock(&TURNS).take().is_some() {
        announce(None);
    }
}

fn announce(status: Option<QueueStatus>) {
    queue::publish(status);
}
