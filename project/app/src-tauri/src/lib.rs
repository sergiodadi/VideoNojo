use std::path::Path;

use serde::Serialize;
use tauri::menu::{MenuBuilder, SubmenuBuilder};
use tauri::Emitter;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use video_collector_export::{
    backup_database, build_delete_plan, build_edit_plan, build_plan, commit_plan,
    ensure_not_real_collection, is_movie_collector_running, load_movies, open_readonly,
    open_writable, preview_add_movie, resolve_db_path, set_cover, AddMoviePreview, Movie,
    MovieFields, ReplaceFlags,
};

/// Shared by every write command: refuses to proceed if Movie Collector
/// itself is currently running (see `is_movie_collector_running`'s own
/// doc comment for why) — checked right after the real-collection
/// refusal, before any backup or write.
fn refuse_if_movie_collector_running() -> Result<(), String> {
    if is_movie_collector_running().map_err(|e| e.to_string())? {
        return Err("Movie Collector is currently running — close it before writing.".to_string());
    }
    Ok(())
}

/// The version shown to the user (header badge, both "About
/// VideoNojo" surfaces), deliberately *not* `CARGO_PKG_VERSION` —
/// `Cargo.toml`'s version bumps on every single internal commit,
/// while this tracks the latest published `.dmg` release tag instead
/// (see MOVIE.md § 2.8/§ 6/§ 8 — MusicNojo got this wrong twice: once
/// by not having the constant at all, once by forgetting to bump it
/// before building a new release). Set to the v1.0.0b1 tag **before**
/// building that release's `.dmg`, not after — **update this by hand
/// at every future release** the same way.
const DISPLAY_VERSION: &str = "1.0.0b3";

/// The app's HTML shell has no build step to burn a version string
/// into it, so the frontend asks for it over IPC instead.
#[tauri::command]
fn app_version() -> String {
    DISPLAY_VERSION.to_string()
}

/// Only collections produced by this exact Movie Collector release are
/// known to match the schema this project's SQL targets — same rule
/// and reasoning as BookNojo's/MusicNojo's own SUPPORTED_*_VERSION.
const SUPPORTED_MVCP_VERSION: &str = "20.2.2.1";

/// Reads the Movie Collector version that created a `.mvcp` package,
/// from its own `preferences.plist` (key `kPreferencesApplicationVersion`).
/// Uses `plutil -extract ... raw` (a single key) rather than
/// `-convert json`, which is known to fail outright on a real
/// collection's `preferences.plist` if any other key in it isn't
/// JSON-representable (confirmed on Book/Music Collector's own files).
fn mvcp_version(mvcp_dir: &Path) -> Option<String> {
    let output = std::process::Command::new("plutil")
        .args(["-extract", "kPreferencesApplicationVersion", "raw", "-o", "-"])
        .arg(mvcp_dir.join("preferences.plist"))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8(output.stdout).ok()?;
    Some(version.trim().to_string())
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
            let Some(local_path) = file.as_path() else {
                let _ = app.emit("collection-picked", file.to_string());
                return;
            };

            // Only a path picked here goes through this check — the
            // bundled sample-collection (no preferences.plist at all)
            // is loaded through a different path at boot and never
            // hits this, same split BookNojo/MusicNojo's own import
            // dialogs have.
            match mvcp_version(local_path) {
                Some(version) if version == SUPPORTED_MVCP_VERSION => {}
                Some(version) => {
                    app.dialog()
                        .message(format!(
                            "This collection was created with Movie Collector {version}. \
                             Only version {SUPPORTED_MVCP_VERSION} is supported — opening cancelled."
                        ))
                        .title("Unsupported version")
                        .kind(MessageDialogKind::Error)
                        .show(|_| {});
                    return;
                }
                None => {
                    app.dialog()
                        .message(
                            "Could not determine which Movie Collector version created this \
                             collection — opening cancelled as a precaution.",
                        )
                        .title("Version could not be verified")
                        .kind(MessageDialogKind::Error)
                        .show(|_| {});
                    return;
                }
            }

            let _ = app.emit("collection-picked", local_path.to_string_lossy().to_string());
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
    refuse_if_movie_collector_running()?;
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
    refuse_if_movie_collector_running()?;
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
    refuse_if_movie_collector_running()?;
    backup_database(path).map_err(|e| e.to_string())?;
    let plan = build_delete_plan(id);
    let conn = open_writable(path).map_err(|e| e.to_string())?;
    commit_plan(&conn, &plan).map_err(|e| e.to_string())
}

/// Native picker for a cover image, same async-callback shape as
/// `pick_collection` (and for the same reason — a blocking pick from
/// a command deadlocks). `set_cover` already does its own
/// real-collection refusal and backup internally (see its own doc
/// comment), so this calls it directly rather than going through a
/// shared "open for write" helper — same pattern `vc_add set-cover`'s
/// CLI already uses. Only the front cover is offered here: the entry
/// point is the stats panel's "no cover" list, which only cares about
/// the front cover movies are missing. Result goes back as an event
/// (not the command's return value) since the picker is asynchronous.
#[tauri::command]
fn pick_and_set_cover(app: tauri::AppHandle, path: String, id: i64) {
    app.dialog()
        .file()
        .set_title("Choose a cover image")
        .add_filter("Image", &["jpg", "jpeg", "png", "gif", "bmp"])
        .pick_file(move |file| {
            let Some(file) = file else { return };
            let Some(local_path) = file.as_path() else { return };
            match set_cover(Path::new(&path), id, Some(local_path), None, false) {
                Ok(()) => {
                    let _ = app.emit("cover-set", id);
                }
                Err(e) => {
                    let _ = app.emit("cover-set-error", e.to_string());
                }
            }
        });
}

fn build_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    #[cfg(target_os = "macos")]
    let app_menu = SubmenuBuilder::new(app, "VideoNojo")
        .about(Some(tauri::menu::AboutMetadata {
            // Overrides the bundle's own CFBundleShortVersionString/
            // CFBundleVersion (still CARGO_PKG_VERSION/Cargo.toml's
            // per-commit dev version) with DISPLAY_VERSION — the
            // native About panel is a separate surface from the
            // custom dialog below, driven by the bundle's own Info.plist
            // fields rather than any Rust command, so both need fixing.
            version: Some(DISPLAY_VERSION.to_string()),
            short_version: Some(DISPLAY_VERSION.to_string()),
            ..Default::default()
        }))
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
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            app_version,
            list_movies,
            pick_collection,
            prepare_add_movie,
            add_movie,
            edit_movie,
            delete_movie,
            pick_and_set_cover
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
                    .message(format!("VideoNojo\nVersion {}", DISPLAY_VERSION))
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

    #[test]
    fn mvcp_version_is_none_without_a_preferences_plist() {
        // The bundled sample-collection has no preferences.plist at
        // all (it's never run through this version check — see
        // pick_collection's own comment on why).
        assert_eq!(mvcp_version(&sample_collection_path()), None);
    }
}
