use std::path::Path;

use serde::Serialize;
use tauri::menu::{MenuBuilder, SubmenuBuilder};
use tauri::Emitter;
use tauri_plugin_dialog::DialogExt;
use video_collector_export::{
    backup_database, build_delete_plan, build_edit_plan, build_plan, commit_plan,
    ensure_not_real_collection, load_movies, open_readonly, open_writable, preview_add_movie,
    resolve_db_path, AddMoviePreview, Movie, MovieFields, ReplaceFlags,
};

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

/// Opens the native file picker for a `.mvcp` collection and, once
/// one is chosen, emits `"collection-picked"` with its path — the
/// frontend listens and reloads via `list_movies`. Uses the dialog
/// plugin's async callback form, never `blocking_*`: a blocking call
/// from inside a `#[tauri::command]` deadlocks on macOS, since the
/// native panel needs the main thread and the command runs on a
/// worker thread.
///
/// A file picker, not a folder picker: `.mvcp` is registered with
/// macOS/LaunchServices as a package (document type) by Movie
/// Collector itself, so a folder-only picker would grey it out —
/// `pick_file()` with an extension filter lets it be selected like
/// any other file.
#[tauri::command]
fn pick_collection(app: tauri::AppHandle) {
    app.dialog()
        .file()
        .set_title("Open a Movie Collector collection (.mvcp)")
        .add_filter("Collection", &["mvcp"])
        .pick_file(move |file| {
            let Some(file) = file else { return };
            let path = match file.as_path() {
                Some(p) => p.to_string_lossy().to_string(),
                None => file.to_string(),
            };
            let _ = app.emit("collection-picked", path);
        });
}

/// Resolves a new movie against the collection, read-only — the
/// frontend shows every resolved field with a "new" badge and
/// requires confirming before `add_movie` actually writes anything.
#[tauri::command]
fn prepare_add_movie(path: String, fields: MovieFields) -> Result<AddMoviePreview, String> {
    let conn = open_readonly(Path::new(&path)).map_err(|e| e.to_string())?;
    preview_add_movie(&conn, &fields).map_err(|e| e.to_string())
}

/// Writes a new movie — only ever reachable after `prepare_add_movie`
/// resolved it and the user confirmed. Same real-collection refusal
/// and pre-write backup as the CLI's own `commit`.
#[tauri::command]
fn add_movie(path: String, fields: MovieFields) -> Result<(), String> {
    let path = Path::new(&path);
    ensure_not_real_collection(path).map_err(|e| e.to_string())?;
    backup_database(path).map_err(|e| e.to_string())?;
    let plan = {
        let conn = open_readonly(path).map_err(|e| e.to_string())?;
        build_plan(&conn, &fields).map_err(|e| e.to_string())?
    };
    let conn = open_writable(path).map_err(|e| e.to_string())?;
    commit_plan(&conn, &plan).map_err(|e| e.to_string())
}

/// Writes changes to an existing movie — same real-collection refusal
/// and pre-write backup as `add_movie`. No preview step (same scope
/// as BookNojo/MusicNojo's own Edit, which also writes directly).
#[tauri::command]
fn edit_movie(path: String, id: i64, fields: MovieFields, replace: ReplaceFlags) -> Result<(), String> {
    let path = Path::new(&path);
    ensure_not_real_collection(path).map_err(|e| e.to_string())?;
    backup_database(path).map_err(|e| e.to_string())?;
    let plan = {
        let conn = open_readonly(path).map_err(|e| e.to_string())?;
        build_edit_plan(&conn, id, &fields, &replace).map_err(|e| e.to_string())?
    };
    let conn = open_writable(path).map_err(|e| e.to_string())?;
    commit_plan(&conn, &plan).map_err(|e| e.to_string())
}

/// Deletes a movie — same real-collection refusal and pre-write
/// backup as `add_movie`/`edit_movie`. See `build_delete_plan`'s own
/// doc comment for exactly what is and isn't cleaned up.
#[tauri::command]
fn delete_movie(path: String, id: i64) -> Result<(), String> {
    let path = Path::new(&path);
    ensure_not_real_collection(path).map_err(|e| e.to_string())?;
    backup_database(path).map_err(|e| e.to_string())?;
    let plan = build_delete_plan(id);
    let conn = open_writable(path).map_err(|e| e.to_string())?;
    commit_plan(&conn, &plan).map_err(|e| e.to_string())
}

fn build_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    #[cfg(target_os = "macos")]
    let app_menu = SubmenuBuilder::new(app, "VideoNojo")
        .about(None)
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .text("open_collection", "Open Collection…")
        .separator()
        .close_window()
        .build()?;

    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let window_menu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .separator()
        .close_window()
        .build()?;

    // A submenu titled exactly "Help" is silently swallowed on macOS by
    // this Tauri version — "VideoNojo Help" sidesteps it, same
    // finding/workaround as BookNojo/MusicNojo's own menu.
    let help_menu = SubmenuBuilder::new(app, "VideoNojo Help")
        .text("about_videonojo", "About VideoNojo")
        .build()?;

    let mut builder = MenuBuilder::new(app);
    #[cfg(target_os = "macos")]
    {
        builder = builder.item(&app_menu);
    }
    builder
        .item(&file_menu)
        .item(&edit_menu)
        .item(&window_menu)
        .item(&help_menu)
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            app_version,
            list_movies,
            pick_collection,
            prepare_add_movie,
            add_movie,
            edit_movie,
            delete_movie
        ])
        .setup(|app| {
            let menu = build_menu(app.handle())?;
            app.set_menu(menu)?;
            Ok(())
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open_collection" => pick_collection(app.clone()),
            "about_videonojo" => {
                app.dialog()
                    .message(format!("VideoNojo\nVersion {}", env!("CARGO_PKG_VERSION")))
                    .title("About VideoNojo")
                    .show(|_| {});
            }
            _ => {}
        })
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
