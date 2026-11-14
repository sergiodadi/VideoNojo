use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use video_collector_export::{
    backup_database, build_delete_plan, build_edit_plan, build_plan, commit_plan,
    ensure_not_real_collection, open_readonly, open_writable, MovieFields, ReplaceFlags,
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
    }
    Ok(())
}
