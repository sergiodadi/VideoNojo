use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use video_collector_export::{load_movies, open_readonly, resolve_db_path, write_html, Connection, ENT_MOVIE};

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

    /// Write a standalone, read-only HTML page to this path.
    #[arg(long)]
    html: Option<PathBuf>,

    /// Only keep the named sub-collection (ZCOLLECTION table, e.g.
    /// "My movie collection"). A .mvcp file can hold several
    /// sub-collections; without this option, ALL of them are exported.
    #[arg(long)]
    collection: Option<String>,

    /// Lists the available sub-collections (name, movie count) and
    /// stops — writes no CSV/JSON/HTML.
    #[arg(long)]
    list_collections: bool,
}

/// Prints every sub-collection with its movie count — same shape as
/// BookNojo's/MusicNojo's own `print_collections`.
fn print_collections(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT col.ZTITLE, COUNT(m.Z_PK)
         FROM ZCOLLECTION col
         LEFT JOIN ZCOLLECTIBLE m ON m.ZCOLLECTION = col.Z_PK AND m.Z_ENT = ?1
         GROUP BY col.Z_PK
         ORDER BY col.ZTITLE",
    )?;
    let mut rows = stmt.query([ENT_MOVIE])?;
    println!("{:<30} {:>10}", "collection", "movies");
    while let Some(row) = rows.next()? {
        let name: String = row.get(0)?;
        let count: i64 = row.get(1)?;
        println!("{:<30} {:>10}", name, count);
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    let conn = open_readonly(&args.db)?;

    if args.list_collections {
        return print_collections(&conn);
    }

    let mut movies = load_movies(&conn)?;
    if let Some(name) = &args.collection {
        movies.retain(|m| m.collection.as_deref() == Some(name.as_str()));
    }

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

    if let Some(html_path) = &args.html {
        let images_dir = resolve_db_path(&args.db)
            .parent()
            .map(|p| p.join("Images"))
            .unwrap_or_default();
        write_html(html_path, &movies, &images_dir)?;
        println!("HTML written to {}", html_path.display());
    }

    Ok(())
}
