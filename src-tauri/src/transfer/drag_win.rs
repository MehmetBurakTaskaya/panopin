//! Dragging items out of the shelf on Windows (D-023): a shell data object carries the staged
//! files and the text, and `SHDoDragDrop` runs the drag with a small drop source of our own.

use std::{mem::ManuallyDrop, path::PathBuf, ptr};

use windows::{
    core::{implement, BOOL, HRESULT, HSTRING},
    Win32::{
        Foundation::{
            GlobalFree, DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, HWND,
            POINT, RECT, S_OK,
        },
        System::{
            Com::{
                CoTaskMemFree, IBindCtx, IDataObject, DVASPECT_CONTENT, FORMATETC, STGMEDIUM,
                STGMEDIUM_0, TYMED_HGLOBAL,
            },
            Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
            Ole::{IDropSource, IDropSource_Impl, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE},
            SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS},
        },
        UI::{
            Shell::{
                Common::ITEMIDLIST, ILFindLastID, SHCreateDataObject, SHDoDragDrop,
                SHParseDisplayName,
            },
            WindowsAndMessaging::GetCursorPos,
        },
    },
};

use crate::error::Result;

const CF_UNICODETEXT: u16 = 13;

/// The board lets the pointer through during a drag, so the app beneath can take the drop.
/// Letting go over the board itself is a change of mind, never a drop into what lies beneath.
#[implement(IDropSource)]
struct DropSource {
    /// The board in screen pixels.
    board: RECT,
}

impl IDropSource_Impl for DropSource_Impl {
    fn QueryContinueDrag(&self, escape: BOOL, keys: MODIFIERKEYS_FLAGS) -> HRESULT {
        if escape.as_bool() {
            return DRAGDROP_S_CANCEL;
        }
        if keys.0 & MK_LBUTTON.0 != 0 {
            return S_OK;
        }
        let mut cursor = POINT::default();
        // SAFETY: GetCursorPos writes into the local point only.
        let over_board = unsafe { GetCursorPos(&mut cursor) }.is_ok()
            && (self.board.left..self.board.right).contains(&cursor.x)
            && (self.board.top..self.board.bottom).contains(&cursor.y);
        if over_board {
            DRAGDROP_S_CANCEL
        } else {
            DRAGDROP_S_DROP
        }
    }

    fn GiveFeedback(&self, _effect: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

/// Runs the drag; must be called on the UI thread while the mouse button is down. `board` is
/// left, top, right, bottom in screen pixels. Returns whether the items were dropped somewhere.
pub fn run(hwnd: isize, board: [i32; 4], text: Option<&[u16]>, files: &[PathBuf]) -> Result<bool> {
    let [left, top, right, bottom] = board;
    let source: IDropSource = DropSource {
        board: RECT {
            left,
            top,
            right,
            bottom,
        },
    }
    .into();
    // SAFETY: item lists are freed after the data object copied them; the text block is handed
    // to the data object, which owns it once SetData succeeds.
    unsafe {
        let data = data_object(files)?;
        if let Some(text) = text {
            set_text(&data, text)?;
        }
        let effect = SHDoDragDrop(Some(HWND(hwnd as *mut _)), &data, &source, DROPEFFECT_COPY)?;
        Ok(effect != DROPEFFECT_NONE)
    }
}

unsafe fn parse(path: &std::path::Path) -> Result<*mut ITEMIDLIST> {
    let mut pidl = ptr::null_mut();
    SHParseDisplayName(&HSTRING::from(path), None::<&IBindCtx>, &mut pidl, 0, None)?;
    Ok(pidl)
}

unsafe fn free(pidls: &[*mut ITEMIDLIST]) {
    for &pidl in pidls {
        CoTaskMemFree(Some(pidl.cast_const().cast()));
    }
}

/// A shell data object for files gives every drop target the formats it expects (file list,
/// shell items); staged files all live in one folder.
unsafe fn data_object(files: &[PathBuf]) -> Result<IDataObject> {
    let Some(folder) = files.first().and_then(|file| file.parent()) else {
        return Ok(SHCreateDataObject(None, None, None::<&IDataObject>)?);
    };
    let mut pidls = vec![parse(folder)?];
    for file in files {
        match parse(file) {
            Ok(pidl) => pidls.push(pidl),
            Err(err) => {
                free(&pidls);
                return Err(err);
            }
        }
    }
    let children: Vec<*const ITEMIDLIST> = pidls[1..]
        .iter()
        .map(|&pidl| ILFindLastID(pidl).cast_const())
        .collect();
    let data = SHCreateDataObject(Some(pidls[0]), Some(&children), None::<&IDataObject>);
    free(&pidls);
    Ok(data?)
}

unsafe fn set_text(data: &IDataObject, text: &[u16]) -> Result<()> {
    let global = GlobalAlloc(GMEM_MOVEABLE, std::mem::size_of_val(text))?;
    let target = GlobalLock(global).cast::<u16>();
    if target.is_null() {
        let err = windows::core::Error::from_thread();
        let _ = GlobalFree(Some(global));
        return Err(err.into());
    }
    ptr::copy_nonoverlapping(text.as_ptr(), target, text.len());
    let _ = GlobalUnlock(global);
    let format = FORMATETC {
        cfFormat: CF_UNICODETEXT,
        ptd: ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };
    let medium = STGMEDIUM {
        tymed: TYMED_HGLOBAL.0 as u32,
        u: STGMEDIUM_0 { hGlobal: global },
        pUnkForRelease: ManuallyDrop::new(None),
    };
    // With fRelease the data object owns the memory from here on.
    if let Err(err) = data.SetData(&format, &medium, true) {
        let _ = GlobalFree(Some(global));
        return Err(err.into());
    }
    Ok(())
}
