use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use video_collector_export::{load_movies, open_readonly};

/// Exports a Movie Collector (.mvcp) collection to CSV/JSON.
#[derive(Parser)]
struct Args {
    /// Path to the .mvcp package (or its Movie_data file directly).
    #[arg(long)]
    db: PathBuf,

    /// Write CSV to this path.
    #[arg(long)]
    csv: Option<PathBuf>,

    /// Write JSON to this path.
    #[arg(long)]
    json: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let conn = open_readonly(&args.db)?;
    let movies = load_movies(&conn)?;

    println!("{} movie(s) read.", movies.len());

    if let Some(json_path) = &args.json {
        let file = std::fs::File::create(json_path)?;
        serde_json::to_writer_pretty(file, &movies)?;
        println!("JSON written to {}", json_path.display());
    }

    if let Some(csv_path) = &args.csv {
        let mut writer = csv::Writer::from_path(csv_path)?;
        writer.write_record([
            "title",
            "title_original",
            "genres",
            "studios",
            "formats",
            "country",
            "release_year",
            "rating",
        ])?;
        for m in &movies {
            writer.write_record([
                m.title.as_str(),
                m.title_original.as_deref().unwrap_or(""),
                &m.genres.join("; "),
                &m.studios.join("; "),
                &m.formats.join("; "),
                m.country.as_deref().unwrap_or(""),
                m.release_year.as_deref().unwrap_or(""),
                &m.rating.map(|r| r.to_string()).unwrap_or_default(),
            ])?;
        }
        writer.flush()?;
        println!("CSV written to {}", csv_path.display());
    }

    Ok(())
}
