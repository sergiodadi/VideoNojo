use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use video_collector_export::{
    backup_database, build_plan, commit_plan, ensure_not_real_collection, open_readonly,
    open_writable, MovieFields,
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
    /// Executes a plan file (from `add-plan`) in one transaction.
    Commit {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        plan: PathBuf,
    },
}

fn main() -> Result<()> {
    match Args::parse().command {
        Command::AddPlan { db, fields, out } => {
            let conn = open_readonly(&db)?;
            let sql = build_plan(&conn, &fields)?;
            match out {
                Some(path) => {
                    std::fs::write(&path, &sql)?;
                    println!("Plan written to {}", path.display());
                }
                None => println!("{sql}"),
            }
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
