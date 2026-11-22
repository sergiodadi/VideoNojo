//! Generates `sample-collection/MyCollection.mvcp`, a tiny, entirely
//! fictional collection committed to the repo for the app's default/
//! demo view — never real personal data. Schema from `schema.sql`
//! (structure only); every movie, person, genre, and studio below is
//! invented. Run with:
//!
//!   cargo run --release --example generate_sample_collection

use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;
use video_collector_export::{build_plan, MovieFields};

const SCHEMA: &str = include_str!("schema.sql");

// Z_ENT, Z_NAME, Z_SUPER — mirrors the real Z_PRIMARYKEY table's rows,
// all starting at Z_MAX=0 since this collection starts empty.
const ENTITIES: &[(i64, &str, i64)] = &[
    (1, "Actor", 0),
    (2, "AudienceRating", 0),
    (3, "AudioTrack", 0),
    (4, "Boxset", 0),
    (5, "Cast", 0),
    (6, "Collectible", 0),
    (7, "Movie", 6),
    (8, "Collection", 0),
    (9, "Condition", 0),
    (10, "Country", 0),
    (11, "CreditPerson", 0),
    (12, "Crew", 0),
    (13, "Disc", 0),
    (14, "Distributor", 0),
    (15, "Edition", 0),
    (16, "Episode", 0),
    (17, "Extra", 0),
    (18, "Feature", 0),
    (19, "Format", 0),
    (20, "Genre", 0),
    (21, "HDR", 0),
    (22, "ItemDelegate", 0),
    (23, "Language", 0),
    (24, "Layers", 0),
    (25, "Link", 0),
    (26, "LocalLink", 25),
    (27, "OnlineLink", 25),
    (28, "Loan", 0),
    (29, "Loaner", 0),
    (30, "Location", 0),
    (31, "LookupItemDelegate", 0),
    (32, "Owner", 0),
    (33, "Packaging", 0),
    (34, "PersonDelegate", 0),
    (35, "Region", 0),
    (36, "ScreenRatio", 0),
    (37, "SearchString", 0),
    (38, "Series", 0),
    (39, "StorageDevice", 0),
    (40, "Store", 0),
    (41, "Studio", 0),
    (42, "Subtitle", 0),
    (43, "Tag", 0),
    (44, "TapeLabel", 0),
    (45, "TapeSpeed", 0),
    (46, "UserDefinedField", 0),
    (47, "UserDefinedLookupItem", 0),
    (48, "UserLookup1", 0),
    (49, "UserLookup2", 0),
];

#[derive(Default)]
struct SampleMovie {
    title: &'static str,
    release_year: &'static str,
    rating: i64,
    notes: &'static str,
    plot: &'static str,
    country: &'static str,
    genres: &'static [&'static str],
    studios: &'static [&'static str],
    formats: &'static [&'static str],
    actors: &'static [&'static str],
    crew: &'static [&'static str],
}

fn sample_movies() -> Vec<SampleMovie> {
    vec![
        SampleMovie {
            title: "The Lighthouse at Dunmoor",
            release_year: "2014",
            rating: 4,
            notes: "Fictional demo entry.",
            plot: "A keeper and his apprentice face a storm that may not be entirely natural.",
            country: "Fictionland",
            genres: &["Drama", "Mystery"],
            studios: &["Harborlight Pictures"],
            formats: &["Blu-ray"],
            actors: &["Nora Fenwick", "Idris Calloway"],
            crew: &["Marguerite Oyelaran:Director", "Tomas Vrba:Writer"],
        },
        SampleMovie {
            title: "Carnival of Rust",
            release_year: "2009",
            rating: 3,
            notes: "",
            plot: "A travelling carnival hides a secret that follows it from town to town.",
            country: "Testland",
            genres: &["Horror", "Thriller"],
            studios: &["Rustbelt Films"],
            formats: &["DVD"],
            actors: &["Paloma Westergaard"],
            crew: &["Benedikt Aalto:Director", "Benedikt Aalto:Writer"],
        },
        SampleMovie {
            title: "Signal from the Orchard",
            release_year: "2021",
            rating: 5,
            notes: "Sample entry — sequel below.",
            plot: "An orchard keeper picks up a radio signal that shouldn't exist.",
            country: "Fictionland",
            genres: &["Science Fiction"],
            studios: &["Harborlight Pictures", "Orchard Collective"],
            formats: &["Blu-ray", "4K UHD"],
            actors: &["Idris Calloway", "Senna Okafor"],
            crew: &["Marguerite Oyelaran:Director", "Senna Okafor:Music"],
        },
        SampleMovie {
            title: "Signal from the Orchard: Harvest",
            release_year: "2023",
            rating: 4,
            notes: "",
            plot: "The sequel to Signal from the Orchard.",
            country: "Fictionland",
            genres: &["Science Fiction"],
            studios: &["Harborlight Pictures"],
            formats: &["4K UHD"],
            actors: &["Idris Calloway"],
            crew: &["Marguerite Oyelaran:Director"],
        },
        SampleMovie {
            title: "Paper Boats and Low Tide",
            release_year: "2017",
            rating: 3,
            notes: "",
            plot: "Two estranged siblings rebuild their father's boatyard over one summer.",
            country: "Testland",
            genres: &["Drama", "Family"],
            studios: &["Rustbelt Films"],
            formats: &["DVD", "Blu-ray"],
            actors: &["Nora Fenwick", "Paloma Westergaard"],
            crew: &["Tomas Vrba:Director", "Tomas Vrba:Producer"],
        },
        SampleMovie {
            title: "The Clockmaker's Daughter",
            release_year: "2005",
            rating: 4,
            notes: "",
            plot: "A watchmaker's apprentice discovers a clock that runs backwards.",
            country: "Fictionland",
            genres: &["Fantasy", "Drama"],
            studios: &["Orchard Collective"],
            formats: &["DVD"],
            actors: &["Senna Okafor", "Idris Calloway"],
            crew: &["Benedikt Aalto:Writer", "Marguerite Oyelaran:Camera"],
        },
        SampleMovie {
            title: "Last Train to Hollow Creek",
            release_year: "2012",
            rating: 2,
            notes: "",
            plot: "A night train's last run of the season takes a detour through a town that no longer exists.",
            country: "Testland",
            genres: &["Horror"],
            studios: &["Rustbelt Films"],
            formats: &["Blu-ray"],
            actors: &["Paloma Westergaard", "Nora Fenwick"],
            crew: &["Benedikt Aalto:Director"],
        },
        SampleMovie {
            title: "Orbital Harvest",
            release_year: "2019",
            rating: 5,
            notes: "",
            plot: "A seed vault in low orbit becomes humanity's last argument about its own future.",
            country: "Fictionland",
            genres: &["Science Fiction", "Drama"],
            studios: &["Orchard Collective"],
            formats: &["4K UHD", "Blu-ray"],
            actors: &["Senna Okafor"],
            crew: &["Marguerite Oyelaran:Director", "Tomas Vrba:Producer", "Senna Okafor:Music"],
        },
    ]
}

fn list(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn opt(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn main() -> Result<()> {
    let root = Path::new("sample-collection/MyCollection.mvcp");
    if root.exists() {
        std::fs::remove_dir_all(root)?;
    }
    std::fs::create_dir_all(root.join("Images"))?;

    let db_path = root.join("Movie_data");
    let conn = Connection::open(&db_path)?;
    conn.execute_batch(SCHEMA)?;

    for (ent, name, super_ent) in ENTITIES {
        conn.execute(
            "INSERT INTO Z_PRIMARYKEY (Z_ENT, Z_NAME, Z_SUPER, Z_MAX) VALUES (?1, ?2, ?3, 0)",
            rusqlite::params![ent, name, super_ent],
        )?;
    }

    let movies = sample_movies();
    let count = movies.len();
    for movie in movies {
        let fields = MovieFields {
            title: Some(movie.title.to_string()),
            release_year: opt(movie.release_year),
            rating: Some(movie.rating),
            notes: opt(movie.notes),
            plot: opt(movie.plot),
            country: opt(movie.country),
            genres: list(movie.genres),
            studios: list(movie.studios),
            formats: list(movie.formats),
            actors: list(movie.actors),
            crew: list(movie.crew),
            ..Default::default()
        };
        let (plan, _pk) = build_plan(&conn, &fields)?;
        conn.execute_batch(&plan)?;
    }

    let _ = std::fs::remove_dir_all(root.join("backups"));

    println!("{count} fictional movies generated in {}", root.display());
    Ok(())
}
