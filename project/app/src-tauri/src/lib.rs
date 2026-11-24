use std::path::Path;

use serde::Serialize;
use tauri::menu::{MenuBuilder, SubmenuBuilder};
use tauri::{Emitter, Manager};
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
const DISPLAY_VERSION: &str = "1.0.0b7";

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

/// Header button "Open Collection…" — opens the native macOS file picker,
/// restricted to an already-imported `.vnj` (mirrors BookNojo's/
/// MusicNojo's own Import/Open split, applied here 24/11/2026: a real
/// Movie Collector `.mvcp` is now only ever a one-shot *source* for
/// "File > Import Collection…", never opened directly here — see
/// `open_import_collection_dialog`). Emits `"collection-picked"` with
/// the chosen path — the frontend listens and reloads via
/// `list_movies`. Uses the dialog plugin's async callback form, never
/// `blocking_*`: a blocking call from inside a `#[tauri::command]`
/// deadlocks on macOS, since the native panel needs the main thread
/// and the command runs on a worker thread. `pick_file()` with an
/// extension filter lets a `.vnj` package be selected as a single
/// atomic item rather than navigated into, the same way it already did
/// for `.mvcp` before this split. No Movie Collector version check
/// needed any more on this path — that only ever applied to a raw
/// `.mvcp`, which this picker can no longer select at all. There is no
/// native file-picker API to show a conditional "empty folder" hint, so
/// the dialog's own title doubles as the help message when no `.vnj`
/// exists yet — shown every time, not just when it's actually empty.
#[tauri::command]
fn pick_collection(app: tauri::AppHandle) {
    app.dialog()
        .file()
        .set_title("Open a VideoNojo collection (.vnj) — no .vnj yet? Use File > Import Collection… first")
        .add_filter("VideoNojo collection", &["vnj"])
        .pick_file(move |file| {
            let Some(file) = file else { return };
            let _ = app.emit("collection-picked", file.to_string());
        });
}

/// Payload for the `collection-imported` event — a brand-new `.vnj` was
/// just bootstrapped from a `.mvcp` (see `open_import_collection_dialog`).
/// `movie_count` lets the frontend show an import summary without a
/// second round trip to count anything itself.
#[derive(Serialize, Clone)]
struct CollectionImported {
    path: String,
    movie_count: i64,
}

/// Small one-button error dialog, factored out because
/// `open_import_collection_dialog` below has three distinct failure
/// points (unsupported/unverifiable Movie Collector version, destination
/// already existing, the copy itself failing) that would otherwise
/// repeat the same `app.dialog()....show(|_| {})` boilerplate three
/// times. Mirrors BookNojo's/MusicNojo's own `show_error_dialog`.
fn show_error_dialog(app: &tauri::AppHandle, title: &str, message: impl Into<String>) {
    app.dialog().message(message.into()).title(title).kind(MessageDialogKind::Error).show(|_| {});
}

/// "File > Import Collection…" — the native macOS menu's own entry
/// point (deliberately *not* in the app's own header/menu: imports are
/// rare, a native-menu-only place suits them, per Serge — same choice as
/// BookNojo/MusicNojo). Distinct from "Open Collection…" (the header
/// button, `pick_collection`): Import only ever takes a real Movie
/// Collector `.mvcp` as source — the picker is filtered accordingly (no
/// `.vnj` here; re-importing an already-VideoNojo database makes no
/// sense) — checks its Movie Collector version exactly like the
/// combined picker used to, then bootstraps a brand-new, independent
/// `.vnj` at the fixed location `default_vnj_destination` computes
/// (`~/Documents/VideoNojo/<name>.vnj`) via the shared
/// `import_collection_to`. The source `.mvcp` is only ever read — this
/// is a one-shot copy, never a live link.
fn open_import_collection_dialog(app: &tauri::AppHandle) {
    let app_handle = app.clone();
    app.dialog()
        .file()
        .set_title("Import a Movie Collector collection (.mvcp)")
        .add_filter("Movie Collector collection", &["mvcp"])
        .pick_file(move |file| {
            let Some(path) = file else { return };
            let Some(source) = path.as_path() else { return };

            match mvcp_version(source) {
                Some(version) if version == SUPPORTED_MVCP_VERSION => {}
                Some(version) => {
                    show_error_dialog(
                        &app_handle,
                        "Unsupported version",
                        format!(
                            "This collection was created with Movie Collector {version}. \
                             Only version {SUPPORTED_MVCP_VERSION} is supported for import \
                             — import cancelled."
                        ),
                    );
                    return;
                }
                None => {
                    show_error_dialog(
                        &app_handle,
                        "Version could not be verified",
                        "Could not determine which Movie Collector version created this \
                         collection — import cancelled as a precaution.",
                    );
                    return;
                }
            }

            let dest = match video_collector_export::default_vnj_destination(source) {
                Ok(dest) => dest,
                Err(e) => {
                    show_error_dialog(&app_handle, "Import failed", format!("{e:#}"));
                    return;
                }
            };

            match video_collector_export::import_collection_to(source, &dest) {
                Ok(outcome) => {
                    let _ = app_handle.emit(
                        "collection-imported",
                        CollectionImported {
                            path: outcome.dest.to_string_lossy().to_string(),
                            movie_count: outcome.movie_count,
                        },
                    );
                }
                Err(e) => {
                    show_error_dialog(&app_handle, "Import failed", format!("{e:#}"));
                }
            }
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
///
/// `cover_path` (optional) comes from the add form's "Choose an
/// image…" button (see `pick_cover_for_new_movie`): once the movie is
/// created and its `Z_PK` known (`build_plan` now returns it alongside
/// the SQL text), the cover is set via `set_cover` — a second,
/// independent write, not the same transaction as the add. If it
/// fails, the movie is still added (recoverable afterwards from the
/// Statistics panel's "no cover" list); the returned message says so.
#[tauri::command]
fn add_movie(path: String, fields: MovieFields, cover_path: Option<String>) -> Result<String, String> {
    let path_ref = Path::new(&path);
    ensure_not_real_collection(path_ref).map_err(|e| e.to_string())?;
    refuse_if_movie_collector_running()?;
    backup_database(path_ref).map_err(|e| e.to_string())?;
    let (plan, movie_pk) = {
        let conn = open_readonly(path_ref).map_err(|e| e.to_string())?;
        build_plan(&conn, &fields).map_err(|e| e.to_string())?
    };
    let conn = open_writable(path_ref).map_err(|e| e.to_string())?;
    commit_plan(&conn, &plan).map_err(|e| e.to_string())?;
    let Some(cover_path) = cover_path else {
        return Ok("Movie added.".to_string());
    };
    match set_cover(path_ref, movie_pk, Some(Path::new(&cover_path)), None, false) {
        Ok(()) => Ok("Movie added with its cover.".to_string()),
        Err(e) => Ok(format!("Movie added, but the cover could not be saved ({e:#}).")),
    }
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

/// Native picker for a cover image to attach to a movie that doesn't
/// exist yet — used by the add-movie form's "Choose an image…"
/// button, before `add_movie` has run at all. Can't write a cover
/// directly (there's no row to attach it to yet), so this only emits
/// the picked path back as an "add-cover-picked" event; the frontend
/// holds onto it and sends it along with the next `add_movie` call.
/// Same async-callback shape as `pick_and_set_cover`/`pick_collection`
/// — no error event, since the only way this doesn't emit is the user
/// cancelling the picker, which isn't an error.
#[tauri::command]
fn pick_cover_for_new_movie(app: tauri::AppHandle) {
    app.dialog()
        .file()
        .set_title("Choose a cover image")
        .add_filter("Image", &["jpg", "jpeg", "png", "gif", "bmp"])
        .pick_file(move |file| {
            let Some(file) = file else { return };
            let Some(local_path) = file.as_path() else { return };
            let _ = app.emit("add-cover-picked", local_path.to_string_lossy().to_string());
        });
}

/// "VideoNojo Help > User Guide" — opens a real help window (a bundled,
/// bilingual, self-contained `help.html`: no Tauri API calls from inside
/// it, so no capability grant needed) instead of the plain name+version
/// dialog this used to show, which only duplicated the real native About
/// panel. Reuses the existing window if already open instead of opening
/// a second one. Same fix as BookNojo's/MusicNojo's own
/// `show_help_window`.
fn show_help_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("help") {
        let _ = win.set_focus();
        return;
    }
    let _ = tauri::WebviewWindowBuilder::new(app, "help", tauri::WebviewUrl::App("help.html".into()))
        .title("VideoNojo Help")
        .inner_size(680.0, 820.0)
        .resizable(true)
        .build();
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
        .text("import_collection", "Import Collection…")
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
    // finding/workaround as BookNojo/MusicNojo's own menu. "User Guide"
    // used to be "About VideoNojo", a bare name+version dialog
    // duplicating the real native About panel (the app_menu's own
    // `.about(...)`) — found by Serge comparing the two, same fix as
    // the other 2 apps. Now opens a real help window instead (see
    // show_help_window).
    let help_menu = SubmenuBuilder::new(app, "VideoNojo Help")
        .text("show_help", "User Guide")
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
            pick_and_set_cover,
            pick_cover_for_new_movie
        ])
        .setup(|app| {
            let menu = build_menu(app.handle())?;
            app.set_menu(menu)?;
            Ok(())
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "import_collection" => open_import_collection_dialog(app),
            "show_help" => show_help_window(app),
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
        // all — it's never run through this version check, which now
        // lives in open_import_collection_dialog only.
        assert_eq!(mvcp_version(&sample_collection_path()), None);
    }

    /// `default_vnj_destination` reads the global `HOME` env var — any
    /// test mutating it must serialize against every other test doing
    /// the same, in this crate's own test binary (same precaution
    /// BookNojo's/MusicNojo's own equivalent tests needed).
    static HOME_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn testdata_mvcp() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/MyCollection.mvcp")
    }

    /// `testdata/` is gitignored/personal — skipped where absent, same
    /// convention the shared crate's own `testdata`-dependent tests use.
    #[test]
    fn mvcp_version_reads_the_supported_version_from_the_real_fixture() {
        let source = testdata_mvcp();
        if !source.exists() {
            eprintln!("skipping: no local testdata/ (run refresh-testdata.sh)");
            return;
        }
        assert_eq!(mvcp_version(&source), Some(SUPPORTED_MVCP_VERSION.to_string()));
    }

    /// Replays, outside of any dialog callback, exactly the sequence
    /// "File > Import Collection…" runs once a `.mvcp` has been picked
    /// (`open_import_collection_dialog`'s body): version check, then
    /// `default_vnj_destination`, then `import_collection_to`. The
    /// dialog itself (native file picker) can't be driven from a test —
    /// mirrors BookNojo's/MusicNojo's own equivalent test exactly.
    #[test]
    fn the_full_import_chain_bootstraps_a_vnj_at_the_fixed_destination() {
        let source = testdata_mvcp();
        if !source.exists() {
            eprintln!("skipping: no local testdata/ (run refresh-testdata.sh)");
            return;
        }
        let _guard = HOME_ENV_LOCK.lock().unwrap();
        let fake_home = std::env::temp_dir().join("videonojo_full_import_chain_test_home");
        let _ = std::fs::remove_dir_all(&fake_home);
        std::fs::create_dir_all(&fake_home).unwrap();
        let previous_home = std::env::var("HOME").ok();
        unsafe { std::env::set_var("HOME", &fake_home) };

        let version = mvcp_version(&source);
        assert_eq!(version, Some(SUPPORTED_MVCP_VERSION.to_string()), "fixture must still match the supported version");

        let dest = video_collector_export::default_vnj_destination(&source).expect("dest computation must succeed");
        assert_eq!(dest, fake_home.join("Documents/VideoNojo/MyCollection.vnj"));

        let outcome = video_collector_export::import_collection_to(&source, &dest).expect("import must succeed");
        assert!(outcome.movie_count > 0);
        assert!(dest.join("Movie_data").is_file());
        assert!(dest.join("Images").is_dir());

        match previous_home {
            Some(home) => unsafe { std::env::set_var("HOME", home) },
            None => unsafe { std::env::remove_var("HOME") },
        }
        let _ = std::fs::remove_dir_all(&fake_home);
    }
}
