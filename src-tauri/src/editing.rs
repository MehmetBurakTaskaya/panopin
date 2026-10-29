//! Edits made in the preview: the body of a text, and the crop and marks over an image (D-050).
//! The original image is never touched; the thumbnail is redrawn from the edited result.

use std::fs;

use tauri::AppHandle;
use tauri::Manager;

use crate::{
    capture::{self, now_ms},
    error::{AppError, Result},
    media::{annotations, annotations::EditLayer, decode, flatten, thumbnails, MediaStore},
    store::edits,
    AppState,
};

fn media(app: &AppHandle) -> Result<MediaStore> {
    capture::media(app).ok_or(AppError::NotReady)
}

pub fn full_text(app: &AppHandle, id: i64) -> Result<String> {
    app.state::<AppState>()
        .with_db(|db| edits::full_text(db, id))
}

pub fn update_text(app: &AppHandle, id: i64, body: &str) -> Result<()> {
    app.state::<AppState>()
        .with_db(|db| edits::update_text(db, id, body))?;
    capture::changed(app);
    Ok(())
}

pub fn load_layer(app: &AppHandle, id: i64) -> Result<Option<EditLayer>> {
    annotations::load(&media(app)?.dir(id))
}

/// Saves the layer and redraws the thumbnail from the flattened result. An empty layer is the
/// same as "revert to original".
pub fn save_layer(app: &AppHandle, id: i64, layer: &EditLayer) -> Result<()> {
    if layer.is_empty() {
        return revert(app, id);
    }
    let media = media(app)?;
    let state = app.state::<AppState>();
    let facts = state.with_db(|db| edits::image_facts(db, id))?;
    layer.validate(facts.width, facts.height)?;
    let original = decode::decode(&fs::read(media.original(id, &facts.media_ext))?)?;
    // Flattening before saving also proves that the layer can be drawn.
    let flat = flatten::flatten(&original, layer)?;
    let thumb = thumbnails::make(&flat)?;
    annotations::save(&media.dir(id), layer)?;
    media.replace_thumb(id, &thumb, facts.thumb_ext.as_deref())?;
    let size = Some(flat.dimensions());
    state.with_db(|db| edits::set_image_edit(db, id, size, thumb.ext, now_ms()))?;
    capture::changed(app);
    Ok(())
}

/// Removes the crop and every mark; the thumbnail is drawn from the original again.
pub fn revert(app: &AppHandle, id: i64) -> Result<()> {
    let media = media(app)?;
    let state = app.state::<AppState>();
    let facts = state.with_db(|db| edits::image_facts(db, id))?;
    let original = decode::decode(&fs::read(media.original(id, &facts.media_ext))?)?;
    let thumb = thumbnails::make(&original)?;
    annotations::remove(&media.dir(id))?;
    media.replace_thumb(id, &thumb, facts.thumb_ext.as_deref())?;
    state.with_db(|db| edits::set_image_edit(db, id, None, thumb.ext, now_ms()))?;
    capture::changed(app);
    Ok(())
}
