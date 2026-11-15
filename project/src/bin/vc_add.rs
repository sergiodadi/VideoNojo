use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use rusqlite::{Connection, OpenFlags};
use video_collector_export::{
    backup_database, build_delete_plan, build_edit_plan, build_plan, commit_plan,
    ensure_not_real_collection, open_readonly, open_writable, resolve_db_path, set_cover,
    MovieFields, ReplaceFlags, ENT_MOVIE,
};

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Resolves a new movie against the collection and prints the SQL
    /// plan — never executes it.
    AddPlan {
        #[arg(long)]
        db: PathBuf,
        #[command(flatten)]
        fields: MovieFields,
        /// Write the plan to this file instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Resolves changes to an existing movie (by Z_PK) and prints the
    /// SQL plan — never executes it.
    EditPlan {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        id: i64,
        #[command(flatten)]
        fields: MovieFields,
        #[command(flatten)]
        replace: ReplaceFlags,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Prints the SQL plan to delete a movie (by Z_PK) — never
    /// executes it. See MOVIE.md for what this does and doesn't clean
    /// up yet.
    DeletePlan {
        #[arg(long)]
        id: i64,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Executes a plan file (from `add-plan`/`edit-plan`/`delete-plan`)
    /// in one transaction.
    Commit {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        plan: PathBuf,
    },
    /// Sets a movie's front and/or back cover image, copying the file
    /// into the collection's Images/ folder.
    SetCover {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        id: i64,
        /// Local image file to use as the front cover
        #[arg(long)]
        front: Option<PathBuf>,
        /// Local image file to use as the back cover
        #[arg(long)]
        back: Option<PathBuf>,
    },
    /// Bootstraps a new, independent ".vnc" database from an existing
    /// collection (package + Images/ folder) — a one-time copy, never
    /// re-run against a destination that already exists. The result
    /// works exactly like a .mvcp (same schema, just a different
    /// extension to mark it as VideoNojo's own going forward, not
    /// Movie Collector's).
    Import {
        /// The existing .mvcp (or any other extension — only the
        /// presence of Movie_data inside matters) to copy from.
        #[arg(long)]
        source: PathBuf,
        /// Defaults to "collection/<source name>.vnc" in the current directory.
        #[arg(long)]
        dest: Option<PathBuf>,
    },
}

/// Default destination when `--dest` is omitted: `collection/<source
/// basename>.vnc` in the current directory — `.vnc` is the chosen
/// extension for the new, independent "VideoNojo" database going
/// forward, same convention as BookNojo's `.col`/MusicNojo's `.mnc`.
fn default_import_dest(source: &Path) -> PathBuf {
    let name = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "MyCollection".to_string());
    PathBuf::from("collection").join(format!("{name}.vnc"))
}

fn cmd_import(source: PathBuf, dest: Option<PathBuf>) -> Result<()> {
    let source_db = resolve_db_path(&source);
    let source_pkg = source_db
        .parent()
        .context("source package folder not found (expected a .mvcp containing Movie_data)")?;

    let dest = dest.unwrap_or_else(|| default_import_dest(&source));
    if dest.exists() {
        bail!(
            "{} already exists — import only bootstraps a brand-new database, once, \
             never to overwrite an existing one. Pass a different --dest, or remove it \
             first if you're sure.",
            dest.display()
        );
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }

    println!("Importing {} to {}...", source_pkg.display(), dest.display());
    let status = std::process::Command::new("cp")
        .args(["-R", &source_pkg.to_string_lossy(), &dest.to_string_lossy()])
        .status()
        .context("running cp -R")?;
    if !status.success() {
        bail!("cp -R failed");
    }
    std::process::Command::new("chmod")
        .args(["-R", "u+w", &dest.to_string_lossy()])
        .status()
        .context("running chmod -R u+w")?;

    let dest_movie_data = dest.join("Movie_data");
    let conn = Connection::open_with_flags(&dest_movie_data, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("opening {}", dest_movie_data.display()))?;
    let movie_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ZCOLLECTIBLE WHERE Z_ENT = ?1",
        [ENT_MOVIE],
        |row| row.get(0),
    )?;

    println!();
    println!("Import complete: {movie_count} movie(s) in {}.", dest.display());
    println!("This database is now independent — no further automatic sync with the");
    println!("source. vc_add/the app can keep writing to it normally — it's a .mvcp");
    println!("like any other, only the extension changed.");

    Ok(())
}

fn write_plan(sql: &str, out: Option<PathBuf>) -> Result<()> {
    match out {
        Some(path) => {
            std::fs::write(&path, sql)?;
            println!("Plan written to {}", path.display());
        }
        None => println!("{sql}"),
    }
    Ok(())
}

fn main() -> Result<()> {
    match Args::parse().command {
        Command::AddPlan { db, fields, out } => {
            let conn = open_readonly(&db)?;
            let sql = build_plan(&conn, &fields)?;
            write_plan(&sql, out)?;
        }
        Command::EditPlan {
            db,
            id,
            fields,
            replace,
            out,
        } => {
            let conn = open_readonly(&db)?;
            let sql = build_edit_plan(&conn, id, &fields, &replace)?;
            write_plan(&sql, out)?;
        }
        Command::DeletePlan { id, out } => {
            let sql = build_delete_plan(id);
            write_plan(&sql, out)?;
        }
        Command::Commit { db, plan } => {
            ensure_not_real_collection(&db)?;
            backup_database(&db)?;
            let sql = std::fs::read_to_string(&plan)?;
            let conn = open_writable(&db)?;
            commit_plan(&conn, &sql)?;
            println!("Committed.");
        }
        Command::SetCover { db, id, front, back } => {
            set_cover(&db, id, front.as_deref(), back.as_deref())?;
        }
        Command::Import { source, dest } => {
            cmd_import(source, dest)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_import_dest_uses_the_vnc_extension_under_collection() {
        assert_eq!(
            default_import_dest(Path::new(
                "/Users/x/Documents/Collectorz/Movie Collector/MyCollection.mvcp"
            )),
            PathBuf::from("collection/MyCollection.vnc")
        );
        assert_eq!(
            default_import_dest(Path::new("Already.vnc")),
            PathBuf::from("collection/Already.vnc")
        );
    }
}
