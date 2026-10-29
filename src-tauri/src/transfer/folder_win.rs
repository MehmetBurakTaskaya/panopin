//! The Windows common item dialog: a folder for exports, files for imports.

use std::path::PathBuf;

use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{ERROR_CANCELLED, HWND},
        System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_INPROC_SERVER},
        UI::Shell::{
            Common::COMDLG_FILTERSPEC, FileOpenDialog, IFileOpenDialog, FOS_ALLOWMULTISELECT,
            FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
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

/// Several files of the given pattern (`*.png;*.mp4`); empty when the user cancels.
pub fn pick_files(owner: isize, title: &str, kind: &str, pattern: &str) -> Result<Vec<PathBuf>> {
    // SAFETY: as in `pick`; the filter strings outlive the call that reads them.
    unsafe {
        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
        dialog.SetOptions(
            dialog.GetOptions()? | FOS_ALLOWMULTISELECT | FOS_FILEMUSTEXIST | FOS_FORCEFILESYSTEM,
        )?;
        dialog.SetTitle(&HSTRING::from(title))?;
        let (kind, pattern) = (HSTRING::from(kind), HSTRING::from(pattern));
        dialog.SetFileTypes(&[COMDLG_FILTERSPEC {
            pszName: windows::core::PCWSTR(kind.as_ptr()),
            pszSpec: windows::core::PCWSTR(pattern.as_ptr()),
        }])?;
        if let Err(err) = dialog.Show(Some(HWND(owner as *mut _))) {
            return if err.code() == ERROR_CANCELLED.to_hresult() {
                Ok(Vec::new())
            } else {
                Err(err.into())
            };
        }
        let results = dialog.GetResults()?;
        let mut paths = Vec::new();
        for index in 0..results.GetCount()? {
            let name = results
                .GetItemAt(index)?
                .GetDisplayName(SIGDN_FILESYSPATH)?;
            if let Ok(path) = name.to_string() {
                paths.push(PathBuf::from(path));
            }
            CoTaskMemFree(Some(name.0.cast_const().cast()));
        }
        Ok(paths)
    }
}
