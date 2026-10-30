//! Screenshot-folder watcher for macOS (D-073): one thread per folder blocks in `kevent` (no
//! polling) and lists the folder again when its entries change. Files are only read, never moved
//! or changed.

use std::{
    collections::HashMap,
    ffi::{CStr, CString, OsString},
    fs, io,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::ffi::OsStrExt,
    },
    path::{Path, PathBuf},
    ptr,
    sync::{mpsc::Sender, Arc},
    thread::{self, JoinHandle},
    time::{Duration, SystemTime},
};

use objc2_core_foundation::{CFPreferencesCopyAppValue, CFString};

use super::{is_image_file, Event};
use crate::error::Result;

/// macOS marks every screenshot it saves with this attribute, whatever the language of its name.
const SCREEN_CAPTURE: &CStr = c"com.apple.metadata:kMDItemIsScreenCapture";
/// The user event that wakes the thread when its watcher is dropped.
const STOP: usize = 1;
/// One pause for pictures that appear without the screenshot mark, in case it comes just after.
const MARK_GRACE: Duration = Duration::from_millis(300);

/// Each visible picture of a folder with its size and time.
type Listing = HashMap<OsString, (u64, Option<SystemTime>)>;

/// Where macOS saves screenshots: the folder chosen in the Screenshot app's options, else the
/// Desktop.
pub fn screenshots_folder() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let chosen = CFPreferencesCopyAppValue(
        &CFString::from_str("location"),
        &CFString::from_str("com.apple.screencapture"),
    )
    .and_then(|value| value.downcast::<CFString>().ok())
    .map(|location| expand_home(&location.to_string(), &home));
    Some(
        chosen
            .filter(|dir| dir.is_dir())
            .unwrap_or_else(|| home.join("Desktop")),
    )
}

fn expand_home(path: &str, home: &Path) -> PathBuf {
    match path.strip_prefix('~') {
        Some(rest) => home.join(rest.trim_start_matches('/')),
        None => PathBuf::from(path),
    }
}

pub struct Watcher {
    queue: Arc<OwnedFd>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Watcher {
    fn drop(&mut self) {
        let wake = event(STOP, libc::EVFILT_USER, 0, libc::NOTE_TRIGGER);
        if change(&self.queue, &wake).is_ok() {
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

/// Watches `dir`. In the system's screenshot folder (`screenshots_only`), which is usually the
/// Desktop, only the pictures macOS marked as screenshots count; in folders the user added, every
/// picture does.
pub fn watch(dir: PathBuf, screenshots_only: bool, tx: Sender<Event>) -> Result<Watcher> {
    // Pictures already there were saved before Panopin watched; only new ones are reported. The
    // list is taken before the watch starts, so a picture saved in between still counts as new.
    let known = listing(&dir);
    let path = CString::new(dir.as_os_str().as_bytes())
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    // SAFETY: plain system calls whose descriptors are owned right away.
    let folder = unsafe { owned(libc::open(path.as_ptr(), libc::O_EVTONLY | libc::O_CLOEXEC))? };
    let queue = unsafe { owned(libc::kqueue())? };
    let entries_changed = event(
        folder.as_raw_fd() as usize,
        libc::EVFILT_VNODE,
        libc::EV_ADD | libc::EV_CLEAR,
        libc::NOTE_WRITE | libc::NOTE_DELETE | libc::NOTE_RENAME | libc::NOTE_REVOKE,
    );
    change(&queue, &entries_changed)?;
    change(
        &queue,
        &event(STOP, libc::EVFILT_USER, libc::EV_ADD | libc::EV_CLEAR, 0),
    )?;
    let queue = Arc::new(queue);
    let thread_queue = Arc::clone(&queue);
    let thread = thread::Builder::new()
        .name("folder-watch".into())
        .spawn(move || run(&thread_queue, folder, &dir, known, screenshots_only, &tx))?;
    Ok(Watcher {
        queue,
        thread: Some(thread),
    })
}

/// # Safety
/// `fd` must be a new descriptor that nothing else owns, or -1 after a failed call.
unsafe fn owned(fd: libc::c_int) -> io::Result<OwnedFd> {
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(OwnedFd::from_raw_fd(fd))
    }
}

fn event(ident: usize, filter: i16, flags: u16, fflags: u32) -> libc::kevent {
    libc::kevent {
        ident,
        filter,
        flags,
        fflags,
        data: 0,
        udata: ptr::null_mut(),
    }
}

/// Registers or triggers one event on the queue.
fn change(queue: &OwnedFd, request: &libc::kevent) -> io::Result<()> {
    // SAFETY: one valid change and no room for events, so the call does not wait.
    let done = unsafe {
        libc::kevent(
            queue.as_raw_fd(),
            request,
            1,
            ptr::null_mut(),
            0,
            ptr::null(),
        )
    };
    if done < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

// `_folder` keeps the watched descriptor open as long as the thread runs.
fn run(
    queue: &OwnedFd,
    _folder: OwnedFd,
    dir: &Path,
    mut known: Listing,
    screenshots_only: bool,
    tx: &Sender<Event>,
) {
    loop {
        let mut got = event(0, 0, 0, 0);
        // SAFETY: room for one event and no timeout: the thread sleeps until something happens.
        let count =
            unsafe { libc::kevent(queue.as_raw_fd(), ptr::null(), 0, &mut got, 1, ptr::null()) };
        if count < 0 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
            continue;
        }
        let (filter, fflags) = (got.filter, got.fflags);
        if count < 0
            || filter == libc::EVFILT_USER
            || fflags & (libc::NOTE_DELETE | libc::NOTE_RENAME | libc::NOTE_REVOKE) != 0
        {
            return; // stopped, or the folder went away
        }
        if count == 0 {
            continue;
        }
        let now = listing(dir);
        let new: Vec<PathBuf> = now
            .iter()
            .filter(|(name, stamp)| known.get(*name) != Some(*stamp))
            .map(|(name, _)| dir.join(name))
            .collect();
        known = now;
        for path in screenshots(new, screenshots_only) {
            if tx.send(Event::File(path)).is_err() {
                return;
            }
        }
    }
}

/// The visible pictures in `dir` with their size and time. macOS writes a screenshot under a
/// hidden name and renames it when it is complete.
fn listing(dir: &Path) -> Listing {
    let Ok(entries) = fs::read_dir(dir) else {
        return HashMap::new();
    };
    entries
        .flatten()
        .filter(|entry| {
            let name = entry.file_name();
            !name.as_bytes().starts_with(b".") && is_image_file(&name.to_string_lossy())
        })
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            meta.is_file()
                .then(|| (entry.file_name(), (meta.len(), meta.modified().ok())))
        })
        .collect()
}

/// The new pictures to report: all of them, or only those marked as screenshots.
fn screenshots(new: Vec<PathBuf>, screenshots_only: bool) -> Vec<PathBuf> {
    if !screenshots_only || new.is_empty() {
        return new;
    }
    let (mut marked, unmarked): (Vec<_>, Vec<_>) =
        new.into_iter().partition(|path| is_screen_capture(path));
    if !unmarked.is_empty() {
        thread::sleep(MARK_GRACE);
        marked.extend(unmarked.into_iter().filter(|path| is_screen_capture(path)));
    }
    marked
}

/// Whether macOS marked the file as a screenshot.
fn is_screen_capture(path: &Path) -> bool {
    let Ok(path) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    // SAFETY: both names end with NUL; without a buffer the call only reports the value's size.
    let size = unsafe {
        libc::getxattr(
            path.as_ptr(),
            SCREEN_CAPTURE.as_ptr(),
            ptr::null_mut(),
            0,
            0,
            0,
        )
    };
    size >= 0
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, sync::mpsc};

    use super::*;

    #[test]
    fn home_is_expanded() {
        let home = Path::new("/Users/someone");
        assert_eq!(
            expand_home("~/Pictures/Screenshots", home),
            home.join("Pictures/Screenshots")
        );
        assert_eq!(expand_home("~", home), home.to_path_buf());
        assert_eq!(
            expand_home("/Volumes/Shots", home),
            PathBuf::from("/Volumes/Shots")
        );
    }

    #[test]
    fn only_new_visible_pictures_are_reported() {
        let dir = std::env::temp_dir().join(format!("panopin-watch-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("before.png"), b"old").unwrap();
        let (tx, rx) = mpsc::channel();
        let watcher = watch(dir.clone(), false, tx).unwrap();
        fs::write(dir.join(".Screenshot.png"), b"partial").unwrap();
        fs::write(dir.join("notes.txt"), b"text").unwrap();
        fs::write(dir.join("new.png"), b"picture").unwrap();
        // A file can be reported twice while it is written; the capture worker skips repeats.
        let mut reported = BTreeSet::new();
        while let Ok(Event::File(path)) = rx.recv_timeout(Duration::from_millis(1500)) {
            reported.insert(path.file_name().unwrap().to_owned());
        }
        assert_eq!(reported, BTreeSet::from([OsString::from("new.png")]));
        drop(watcher); // returns once the thread has stopped
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn only_marked_pictures_count_as_screenshots() {
        let dir = std::env::temp_dir().join(format!("panopin-marks-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let plain = dir.join("photo.png");
        let shot = dir.join("shot.png");
        fs::write(&plain, b"picture").unwrap();
        fs::write(&shot, b"picture").unwrap();
        let path = CString::new(shot.as_os_str().as_bytes()).unwrap();
        let value = b"true";
        // SAFETY: NUL-terminated names and a buffer of the given length.
        let set = unsafe {
            libc::setxattr(
                path.as_ptr(),
                SCREEN_CAPTURE.as_ptr(),
                value.as_ptr().cast(),
                value.len(),
                0,
                0,
            )
        };
        assert_eq!(set, 0);
        assert!(is_screen_capture(&shot));
        assert!(!is_screen_capture(&plain));
        assert_eq!(
            screenshots(vec![plain.clone(), shot.clone()], true),
            vec![shot.clone()]
        );
        assert_eq!(
            screenshots(vec![plain.clone(), shot.clone()], false),
            vec![plain, shot]
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
