//! Screenshot-folder watcher (D-007, D-008): one thread per folder blocks in
//! `ReadDirectoryChangesW` (no polling). Files are only read, never moved or changed.

use std::{
    io,
    os::windows::ffi::OsStrExt,
    path::PathBuf,
    sync::mpsc::Sender,
    thread::{self, JoinHandle},
};

use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        Storage::FileSystem::{
            CreateFileW, ReadDirectoryChangesW, FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY,
            FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE,
            FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::{Com::CoTaskMemFree, IO::CancelIoEx},
        UI::Shell::{
            FOLDERID_Screenshots, FOLDERID_Videos, SHGetKnownFolderPath, KF_FLAG_CREATE,
            KF_FLAG_DEFAULT,
        },
    },
};

use super::{is_capture_file, Event};
use crate::error::Result;

const FILE_ACTION_ADDED: u32 = 1;
const FILE_ACTION_MODIFIED: u32 = 3;
const FILE_ACTION_RENAMED_NEW_NAME: u32 = 5;

/// The user's Screenshots known folder, wherever it lives (OneDrive, localised names, ...).
pub fn screenshots_folder() -> Option<PathBuf> {
    unsafe {
        let path = SHGetKnownFolderPath(&FOLDERID_Screenshots, KF_FLAG_CREATE, None).ok()?;
        let folder = path.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(path.0.cast_const().cast()));
        folder
    }
}

/// Where the Snipping Tool saves screen recordings: `Screen Recordings` in the Videos known
/// folder. It is created when missing, so the first recording is not missed.
pub fn recordings_folder() -> Option<PathBuf> {
    let videos = unsafe {
        let path = SHGetKnownFolderPath(&FOLDERID_Videos, KF_FLAG_DEFAULT, None).ok()?;
        let folder = path.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(path.0.cast_const().cast()));
        folder
    }?;
    let folder = videos.join("Screen Recordings");
    std::fs::create_dir_all(&folder).ok()?;
    Some(folder)
}

struct RawHandle(HANDLE);
// The handle is only used to cancel and close the watch from the owning `Watcher`.
unsafe impl Send for RawHandle {}

pub struct Watcher {
    handle: RawHandle,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Watcher {
    fn drop(&mut self) {
        unsafe {
            let _ = CancelIoEx(self.handle.0, None);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        unsafe {
            let _ = CloseHandle(self.handle.0);
        }
    }
}

pub fn watch(dir: PathBuf, tx: Sender<Event>) -> Result<Watcher> {
    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_LIST_DIRECTORY.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )
    }
    .map_err(io::Error::from)?;
    let raw = RawHandle(handle);
    let thread_handle = RawHandle(handle);
    let thread = thread::Builder::new()
        .name("folder-watch".into())
        .spawn(move || run(thread_handle, dir, tx))?;
    Ok(Watcher {
        handle: raw,
        thread: Some(thread),
    })
}

fn run(handle: RawHandle, dir: PathBuf, tx: Sender<Event>) {
    let mut buffer = vec![0u32; 16 * 1024]; // 64 KiB, DWORD-aligned as the API requires
    loop {
        let mut returned = 0u32;
        let read = unsafe {
            ReadDirectoryChangesW(
                handle.0,
                buffer.as_mut_ptr().cast(),
                (buffer.len() * 4) as u32,
                false,
                FILE_NOTIFY_CHANGE_FILE_NAME
                    | FILE_NOTIFY_CHANGE_SIZE
                    | FILE_NOTIFY_CHANGE_LAST_WRITE,
                Some(&mut returned),
                None,
                None,
            )
        };
        if read.is_err() {
            return; // cancelled, or the folder went away
        }
        let bytes =
            unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), returned as usize) };
        for name in changed_names(bytes) {
            if is_capture_file(&name) && tx.send(Event::File(dir.join(name))).is_err() {
                return;
            }
        }
    }
}

/// File names from a `FILE_NOTIFY_INFORMATION` chain that were added, renamed in or rewritten.
fn changed_names(bytes: &[u8]) -> Vec<String> {
    let read_u32 = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let mut names = Vec::new();
    let mut offset = 0usize;
    while let (Some(next), Some(action), Some(len)) =
        (read_u32(offset), read_u32(offset + 4), read_u32(offset + 8))
    {
        let start = offset + 12;
        let Some(raw) = bytes.get(start..start + len as usize) else {
            break;
        };
        if matches!(
            action,
            FILE_ACTION_ADDED | FILE_ACTION_MODIFIED | FILE_ACTION_RENAMED_NEW_NAME
        ) {
            let units: Vec<u16> = raw
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect();
            names.push(String::from_utf16_lossy(&units));
        }
        if next == 0 {
            break;
        }
        offset += next as usize;
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(next: u32, action: u32, name: &str) -> Vec<u8> {
        let units: Vec<u8> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&next.to_le_bytes());
        bytes.extend_from_slice(&action.to_le_bytes());
        bytes.extend_from_slice(&(units.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&units);
        bytes
    }

    #[test]
    fn notification_chains_are_parsed() {
        let first = entry(0, FILE_ACTION_ADDED, "Ekran görüntüsü 1.png");
        let next = first.len().next_multiple_of(4) as u32;
        let mut chain = entry(next, FILE_ACTION_ADDED, "Ekran görüntüsü 1.png");
        chain.resize(next as usize, 0);
        chain.extend(entry(0, 2 /* removed */, "old.png"));
        assert_eq!(
            changed_names(&chain),
            vec!["Ekran görüntüsü 1.png".to_owned()]
        );
        assert!(changed_names(&[]).is_empty());
    }
}
