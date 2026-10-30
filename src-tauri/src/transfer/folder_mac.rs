//! The macOS open panel (D-073): a folder for exports and watched folders, files for imports.

use std::path::PathBuf;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSModalResponseOK, NSOpenPanel};
use objc2_foundation::{NSArray, NSString};
use objc2_uniform_type_identifiers::UTType;

use crate::error::{AppError, Result};

fn panel(title: &str) -> Result<objc2::rc::Retained<NSOpenPanel>> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| AppError::Mac("panels need the main thread".into()))?;
    let panel = NSOpenPanel::openPanel(mtm);
    // macOS shows the message inside the panel; its title bar has none.
    panel.setMessage(Some(&NSString::from_str(title)));
    Ok(panel)
}

/// Shows the panel; must run on the main thread. `None` when the user cancels.
pub fn pick(title: &str) -> Result<Option<PathBuf>> {
    let panel = panel(title)?;
    panel.setCanChooseFiles(false);
    panel.setCanChooseDirectories(true);
    panel.setCanCreateDirectories(true);
    panel.setAllowsMultipleSelection(false);
    if panel.runModal() != NSModalResponseOK {
        return Ok(None);
    }
    Ok(panel.URL().and_then(|url| url.to_file_path()))
}

/// Several files with the given extensions (`["png", "mp4"]`); empty when the user cancels.
pub fn pick_files(title: &str, extensions: &[&str]) -> Result<Vec<PathBuf>> {
    let panel = panel(title)?;
    panel.setCanChooseFiles(true);
    panel.setCanChooseDirectories(false);
    panel.setAllowsMultipleSelection(true);
    let types: Vec<_> = extensions
        .iter()
        .filter_map(|ext| UTType::typeWithFilenameExtension(&NSString::from_str(ext)))
        .collect();
    panel.setAllowedContentTypes(&NSArray::from_retained_slice(&types));
    if panel.runModal() != NSModalResponseOK {
        return Ok(Vec::new());
    }
    Ok(panel
        .URLs()
        .iter()
        .filter_map(|url| url.to_file_path())
        .collect())
}
