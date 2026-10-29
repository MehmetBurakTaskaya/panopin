//! The Windows folder picker for exports (the common item dialog in folder mode).

use std::path::PathBuf;

use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{ERROR_CANCELLED, HWND},
        System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_INPROC_SERVER},
        UI::Shell::{
            FileOpenDialog, IFileOpenDialog, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS,
            SIGDN_FILESYSPATH,
        },
    },
};

use crate::error::Result;

/// Shows the dialog over `owner`; must run on the UI thread. `None` when the user cancels.
pub fn pick(owner: isize, title: &str) -> Result<Option<PathBuf>> {
    // SAFETY: COM is initialised on the UI thread; the returned path string is freed after copying.
    unsafe {
        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
        dialog.SetOptions(dialog.GetOptions()? | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM)?;
        dialog.SetTitle(&HSTRING::from(title))?;
        if let Err(err) = dialog.Show(Some(HWND(owner as *mut _))) {
            return if err.code() == ERROR_CANCELLED.to_hresult() {
                Ok(None)
            } else {
                Err(err.into())
            };
        }
        let name = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
        let path = name.to_string();
        CoTaskMemFree(Some(name.0.cast_const().cast()));
        Ok(path.ok().map(PathBuf::from))
    }
}
