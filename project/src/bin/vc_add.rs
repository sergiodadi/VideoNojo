use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use rusqlite::{Connection, OpenFlags};
use video_collector_export::{
    backup_database, build_delete_plan, build_edit_plan, build_plan, commit_plan,
    is_movie_collector_running, looks_like_real_collection, open_readonly, open_writable,
    resolve_db_path, set_cover, MovieFields, ReplaceFlags, ENT_MOVIE,
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
        /// Allows writing even if --db looks like the real Movie Collector
        /// collection — a typed "YES" confirmation is still required.
        #[arg(long)]
        force: bool,
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
        /// Allows writing even if --db looks like the real Movie Collector
        /// collection — a typed "YES" confirmation is still required.
        #[arg(long)]
        force: bool,
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

/// Second, independent gate on top of `--force`: even with the flag set,
/// a real write to what looks like the real collection requires typing
/// "YES" interactively. Takes `input`/`out` rather than touching stdin/
/// stdout directly so the prompt logic itself is unit-testable without any
/// real terminal or file I/O — same shape as BookNojo's own
/// `confirm_real_write`.
fn confirm_real_write<R: std::io::BufRead, W: std::io::Write>(
    input: &mut R,
    out: &mut W,
    db_path: &Path,
    real_prefix: &Path,
) -> Result<()> {
    writeln!(
        out,
        "⚠️  WARNING: {} is located under {} — this is the REAL Movie Collector \
         collection, not a test copy.",
        db_path.display(),
        real_prefix.display()
    )?;
    writeln!(out, "   A timestamped backup will be taken before writing, but this")?;
    writeln!(out, "   operation directly modifies your real data.")?;
    write!(out, "   Type exactly YES to continue (anything else cancels): ")?;
    out.flush()?;
    let mut answer = String::new();
    input.read_line(&mut answer)?;
    if answer.trim() == "YES" {
        Ok(())
    } else {
        bail!("Confirmation refused or incorrect (expected exactly: YES) — nothing was written.");
    }
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
        Command::Commit { db, plan, force } => {
            match (force, looks_like_real_collection(&db)) {
                (false, Some(real_prefix)) => {
                    bail!(
                        "{} is located under {} — this looks like the real collection.\n\
                         This command writes for real: use a dedicated copy instead, or pass \
                         --force if you really know what you're doing.",
                        db.display(),
                        real_prefix.display()
                    );
                }
                (true, Some(real_prefix)) => {
                    let stdin = std::io::stdin();
                    let mut locked = stdin.lock();
                    let mut stdout = std::io::stdout();
                    confirm_real_write(&mut locked, &mut stdout, &db, &real_prefix)?;
                }
                (_, None) => {}
            }
            if is_movie_collector_running()? {
                bail!("Movie Collector is currently running — close it before writing.");
            }
            backup_database(&db)?;
            let sql = std::fs::read_to_string(&plan)?;
            let conn = open_writable(&db)?;
            commit_plan(&conn, &sql)?;
            println!("Committed.");
        }
        Command::SetCover { db, id, front, back, force } => {
            match (force, looks_like_real_collection(&db)) {
                (false, Some(real_prefix)) => {
                    bail!(
                        "{} is located under {} — this looks like the real collection.\n\
                         This command writes for real: use a dedicated copy instead, or pass \
                         --force if you really know what you're doing.",
                        db.display(),
                        real_prefix.display()
                    );
                }
                (true, Some(real_prefix)) => {
                    let stdin = std::io::stdin();
                    let mut locked = stdin.lock();
                    let mut stdout = std::io::stdout();
                    confirm_real_write(&mut locked, &mut stdout, &db, &real_prefix)?;
                }
                (_, None) => {}
            }
            set_cover(&db, id, front.as_deref(), back.as_deref(), force)?;
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

    fn confirm(answer: &str) -> Result<()> {
        let mut input = std::io::Cursor::new(answer.as_bytes());
        let mut out = Vec::new();
        confirm_real_write(&mut input, &mut out, Path::new("/x/Documents/Collectorz/y"), Path::new("/x/Documents/Collectorz"))
    }

    #[test]
    fn confirm_real_write_accepts_only_an_exact_yes() {
        assert!(confirm("YES\n").is_ok());
        // Trailing whitespace from the terminal is forgiven...
        assert!(confirm("YES \n").is_ok());
        // ...but anything else — including a casual lowercase or a plain
        // Enter — must refuse. This is the safety gate: it must never
        // succeed by accident.
        assert!(confirm("yes\n").is_err());
        assert!(confirm("y\n").is_err());
        assert!(confirm("\n").is_err());
        assert!(confirm("").is_err());
    }
}
