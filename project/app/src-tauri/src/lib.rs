use std::path::Path;

use serde::Serialize;
use video_collector_export::{load_movies, open_readonly, resolve_db_path, Movie};

/// The app's HTML shell has no build step to burn a version string
/// into it, so the frontend asks for it over IPC instead.
#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Absolute path to the bundled fictional sample collection, used
/// when the frontend hasn't opened a real one yet.
fn sample_collection_path() -> std::path::PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sample-collection/MyCollection.mvcp");
    dir.canonicalize().unwrap_or(dir)
}

fn resolve_collection_path(path: Option<String>) -> std::path::PathBuf {
    let resolved = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(sample_collection_path);
    resolved.canonicalize().unwrap_or(resolved)
}

#[derive(Serialize)]
struct MoviesResponse {
    movies: Vec<Movie>,
    /// The actual collection path used — the frontend starts with no
    /// path (loads the bundled sample collection by default) and must
    /// learn the real one to pass back to any future write command.
    path: String,
    /// Absolute path to this collection's Images/ folder, for cover
    /// display once covers are modeled on the read/write side.
    images_dir: String,
}

#[tauri::command]
fn list_movies(app: tauri::AppHandle, path: Option<String>) -> Result<MoviesResponse, String> {
    list_movies_impl(&app, path)
}

/// Generic over the Tauri runtime so it can be exercised in a unit
/// test against `tauri::test::mock_app()`'s `MockRuntime`, without
/// needing a real window.
fn list_movies_impl<R: tauri::Runtime>(
    app: &impl tauri::Manager<R>,
    path: Option<String>,
) -> Result<MoviesResponse, String> {
    let path = resolve_collection_path(path);
    let db_path = resolve_db_path(&path);
    grant_asset_scope_for_collection(app, &db_path);

    let conn = open_readonly(&path).map_err(|e| e.to_string())?;
    let movies = load_movies(&conn).map_err(|e| e.to_string())?;
    let images_dir = db_path
        .parent()
        .map(|p| p.join("Images").to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(MoviesResponse {
        movies,
        path: path.to_string_lossy().to_string(),
        images_dir,
    })
}

/// Grants the asset protocol read access to whichever collection's
/// `Images/` folder was just opened — dynamic per collection, never a
/// static dev-machine path list (which would silently fail for any
/// real user's collection elsewhere).
fn grant_asset_scope_for_collection<R: tauri::Runtime>(app: &impl tauri::Manager<R>, db_path: &Path) {
    if let Some(images_dir) = db_path.parent().map(|p| p.join("Images")) {
        let _ = app.asset_protocol_scope().allow_directory(&images_dir, true);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![app_version, list_movies])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_movies_with_no_path_returns_a_usable_path() {
        let app = tauri::test::mock_app();
        let result = list_movies_impl(app.handle(), None).unwrap();
        assert!(!result.movies.is_empty());
        assert!(result.path.contains("sample-collection"));
    }
}
