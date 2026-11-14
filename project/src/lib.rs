use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
pub use rusqlite::Connection;
use serde::Serialize;

// Core Data entity id for Movie, the only entity actually stored in
// ZCOLLECTIBLE in practice (confirmed directly: every row in a real
// collection is Z_ENT=7, Z_SUPER=6 under the abstract "Collectible"
// root). Unlike Book/Music Collector, the other member-like entities
// (Disc/Episode/Boxset/Series/Feature) are NOT part of this hierarchy
// at all — each is its own top-level entity with its own table.
pub const ENT_MOVIE: i64 = 7;

#[derive(Serialize, Clone)]
pub struct Credit {
    pub name: String,
    // ZCREW.ZROLEID: no label exists anywhere in Movie_data itself —
    // left unresolved, not guessed. Same situation as Book/Music
    // Collector's own generic-credit role id.
    pub role_id: Option<i64>,
}

#[derive(Serialize, Clone)]
pub struct CastMember {
    pub name: String,
    pub character: Option<String>,
}

#[derive(Serialize)]
pub struct Movie {
    pub id: i64,
    pub title: String,
    pub title_original: Option<String>,
    pub cast: Vec<CastMember>,
    pub crew: Vec<Credit>,
    pub genres: Vec<String>,
    pub studios: Vec<String>,
    pub formats: Vec<String>,
    pub country: Option<String>,
    pub condition: Option<String>,
    pub packaging: Option<String>,
    pub distributor: Option<String>,
    pub edition: Option<String>,
    pub location: Option<String>,
    pub owner: Option<String>,
    pub audience_rating: Option<String>,
    pub language: Option<String>,
    pub collection: Option<String>,
    /// "YYYY-MM-DD HH:MM:SS", UTC (ZDATEADDED is a Core Data
    /// reference-date timestamp: seconds since 2001-01-01, converted
    /// in SQL).
    pub date_added: Option<String>,
    pub release_year: Option<String>,
    pub rating: Option<i64>,
    pub notes: Option<String>,
    pub plot: Option<String>,
    pub upc: Option<String>,
    pub front_cover: Option<String>,
    pub back_cover: Option<String>,
}

/// Opens a `.mvcp` package (or its `Movie_data` file directly) read-only.
pub fn open_readonly(db: &Path) -> Result<Connection> {
    let db_path = resolve_db_path(db);
    Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("opening {}", db_path.display()))
}

pub fn resolve_db_path(path: &Path) -> std::path::PathBuf {
    if path.is_dir() {
        path.join("Movie_data")
    } else {
        path.to_path_buf()
    }
}

/// Every lookup kind in Movie Collector is its own thin entity
/// (`{Z_PK, Z_ENT, Z_OPT, ZDELEGATE, ZITEMDELEGATE}`) whose `ZDELEGATE`
/// points at `ZLOOKUPITEMDELEGATE.Z_PK` for the actual display name —
/// one extra hop compared to Book/Music Collector, where the
/// collectible's own FK points at `ZLOOKUPITEMDELEGATE` directly.
/// `table` is the lookup's own table name, e.g. `"ZCOUNTRY"`.
fn load_lookup_map(conn: &Connection, table: &str) -> Result<HashMap<i64, String>> {
    let sql = format!(
        "SELECT t.Z_PK, d.ZDISPLAYNAME FROM {table} t \
         JOIN ZLOOKUPITEMDELEGATE d ON t.ZDELEGATE = d.Z_PK"
    );
    let mut stmt = conn.prepare(&sql)?;
    let map = stmt
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(map)
}

/// Same one-extra-hop shape as `load_lookup_map`, but for person-like
/// entities (`Actor`, `CreditPerson`), resolved via `ZPERSONDELEGATE`
/// instead of `ZLOOKUPITEMDELEGATE`.
fn load_person_map(conn: &Connection, table: &str) -> Result<HashMap<i64, String>> {
    let sql = format!(
        "SELECT t.Z_PK, d.ZDISPLAYNAME FROM {table} t \
         JOIN ZPERSONDELEGATE d ON t.ZDELEGATE = d.Z_PK"
    );
    let mut stmt = conn.prepare(&sql)?;
    let map = stmt
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(map)
}

/// A many-to-many Movie↔lookup join, e.g. `join_table = "Z_20MOVIES"`,
/// `join_movie_col = "Z_7MOVIES3"`, `join_lookup_col = "Z_20GENRES"`,
/// `lookup_table = "ZGENRE"`. Returns movie Z_PK -> sorted display names.
fn load_many_to_many(
    conn: &Connection,
    join_table: &str,
    join_movie_col: &str,
    join_lookup_col: &str,
    lookup_table: &str,
) -> Result<HashMap<i64, Vec<String>>> {
    let sql = format!(
        "SELECT j.{join_movie_col}, d.ZDISPLAYNAME \
         FROM {join_table} j \
         JOIN {lookup_table} t ON j.{join_lookup_col} = t.Z_PK \
         JOIN ZLOOKUPITEMDELEGATE d ON t.ZDELEGATE = d.Z_PK"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut map: HashMap<i64, Vec<String>> = HashMap::new();
    let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?;
    for row in rows.filter_map(|r| r.ok()) {
        map.entry(row.0).or_default().push(row.1);
    }
    for names in map.values_mut() {
        names.sort();
    }
    Ok(map)
}

struct Lookups {
    countries: HashMap<i64, String>,
    conditions: HashMap<i64, String>,
    packagings: HashMap<i64, String>,
    distributors: HashMap<i64, String>,
    editions: HashMap<i64, String>,
    locations: HashMap<i64, String>,
    owners: HashMap<i64, String>,
    audience_ratings: HashMap<i64, String>,
    languages: HashMap<i64, String>,
    collections: HashMap<i64, String>,
    actors: HashMap<i64, String>,
    credit_people: HashMap<i64, String>,
    genres: HashMap<i64, Vec<String>>,
    studios: HashMap<i64, Vec<String>>,
    formats: HashMap<i64, Vec<String>>,
}

// ZCOLLECTION stores ZTITLE directly, no delegate hop — same shape as
// Book/Music Collector's own ZCOLLECTION.
fn load_collections_map(conn: &Connection) -> Result<HashMap<i64, String>> {
    let mut stmt = conn.prepare("SELECT Z_PK, ZTITLE FROM ZCOLLECTION")?;
    let map = stmt
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(map)
}

fn load_lookups(conn: &Connection) -> Result<Lookups> {
    Ok(Lookups {
        countries: load_lookup_map(conn, "ZCOUNTRY")?,
        conditions: load_lookup_map(conn, "ZCONDITION")?,
        packagings: load_lookup_map(conn, "ZPACKAGING")?,
        distributors: load_lookup_map(conn, "ZDISTRIBUTOR")?,
        editions: load_lookup_map(conn, "ZEDITION")?,
        locations: load_lookup_map(conn, "ZLOCATION")?,
        owners: load_lookup_map(conn, "ZOWNER")?,
        audience_ratings: load_lookup_map(conn, "ZAUDIENCERATING")?,
        languages: load_lookup_map(conn, "ZLANGUAGE")?,
        collections: load_collections_map(conn)?,
        actors: load_person_map(conn, "ZACTOR")?,
        credit_people: load_person_map(conn, "ZCREDITPERSON")?,
        genres: load_many_to_many(conn, "Z_20MOVIES", "Z_7MOVIES3", "Z_20GENRES", "ZGENRE")?,
        studios: load_many_to_many(conn, "Z_7STUDIOS", "Z_7MOVIES7", "Z_41STUDIOS", "ZSTUDIO")?,
        formats: load_many_to_many(conn, "Z_19MOVIES", "Z_7MOVIES2", "Z_19FORMATS", "ZFORMAT")?,
    })
}

fn load_cast(conn: &Connection, lookups: &Lookups) -> Result<HashMap<i64, Vec<CastMember>>> {
    let mut stmt = conn.prepare("SELECT ZMOVIE, ZACTOR, ZCHARACTER, ZRANK FROM ZCAST ORDER BY ZMOVIE, ZRANK")?;
    let mut map: HashMap<i64, Vec<CastMember>> = HashMap::new();
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, Option<i64>>(0)?,
            row.get::<_, Option<i64>>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;
    for (movie_pk, actor_pk, character) in rows.filter_map(|r| r.ok()) {
        let (Some(movie_pk), Some(actor_pk)) = (movie_pk, actor_pk) else {
            continue;
        };
        let Some(name) = lookups.actors.get(&actor_pk) else {
            continue;
        };
        map.entry(movie_pk).or_default().push(CastMember {
            name: name.clone(),
            character,
        });
    }
    Ok(map)
}

fn load_crew(conn: &Connection, lookups: &Lookups) -> Result<HashMap<i64, Vec<Credit>>> {
    let mut stmt = conn.prepare("SELECT ZMOVIE, ZCREDITPERSON, ZROLEID, ZRANK FROM ZCREW ORDER BY ZMOVIE, ZRANK")?;
    let mut map: HashMap<i64, Vec<Credit>> = HashMap::new();
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, Option<i64>>(0)?,
            row.get::<_, Option<i64>>(1)?,
            row.get::<_, Option<i64>>(2)?,
        ))
    })?;
    for (movie_pk, person_pk, role_id) in rows.filter_map(|r| r.ok()) {
        let (Some(movie_pk), Some(person_pk)) = (movie_pk, person_pk) else {
            continue;
        };
        let Some(name) = lookups.credit_people.get(&person_pk) else {
            continue;
        };
        map.entry(movie_pk).or_default().push(Credit {
            name: name.clone(),
            role_id,
        });
    }
    Ok(map)
}

pub fn load_movies(conn: &Connection) -> Result<Vec<Movie>> {
    let lookups = load_lookups(conn)?;
    let cast_by_movie = load_cast(conn, &lookups)?;
    let crew_by_movie = load_crew(conn, &lookups)?;

    let mut stmt = conn.prepare(
        "SELECT Z_PK, ZTITLE, ZTITLEORIGINAL, ZCOUNTRY, ZCONDITION, ZPACKAGING, \
                ZDISTRIBUTOR, ZEDITION, ZLOCATION, ZOWNER, ZAUDIENCERATING, \
                ZLANGUAGE, ZCOLLECTION, ZMYRATING, ZNOTES, ZPLOT, ZUPC, \
                ZFRONTCOVER, ZBACKCOVER, ZRELEASEYEAR, \
                datetime(ZDATEADDED + 978307200, 'unixepoch') \
         FROM ZCOLLECTIBLE WHERE Z_ENT = ?1 ORDER BY Z_PK",
    )?;

    let movies = stmt
        .query_map([ENT_MOVIE], |row| {
            let id: i64 = row.get(0)?;
            let country_pk: Option<i64> = row.get(3)?;
            let condition_pk: Option<i64> = row.get(4)?;
            let packaging_pk: Option<i64> = row.get(5)?;
            let distributor_pk: Option<i64> = row.get(6)?;
            let edition_pk: Option<i64> = row.get(7)?;
            let location_pk: Option<i64> = row.get(8)?;
            let owner_pk: Option<i64> = row.get(9)?;
            let audience_rating_pk: Option<i64> = row.get(10)?;
            let language_pk: Option<i64> = row.get(11)?;
            let collection_pk: Option<i64> = row.get(12)?;

            Ok(Movie {
                id,
                title: row.get(1)?,
                title_original: row.get(2)?,
                cast: cast_by_movie.get(&id).cloned().unwrap_or_default(),
                crew: crew_by_movie.get(&id).cloned().unwrap_or_default(),
                genres: lookups.genres.get(&id).cloned().unwrap_or_default(),
                studios: lookups.studios.get(&id).cloned().unwrap_or_default(),
                formats: lookups.formats.get(&id).cloned().unwrap_or_default(),
                country: country_pk.and_then(|pk| lookups.countries.get(&pk).cloned()),
                condition: condition_pk.and_then(|pk| lookups.conditions.get(&pk).cloned()),
                packaging: packaging_pk.and_then(|pk| lookups.packagings.get(&pk).cloned()),
                distributor: distributor_pk.and_then(|pk| lookups.distributors.get(&pk).cloned()),
                edition: edition_pk.and_then(|pk| lookups.editions.get(&pk).cloned()),
                location: location_pk.and_then(|pk| lookups.locations.get(&pk).cloned()),
                owner: owner_pk.and_then(|pk| lookups.owners.get(&pk).cloned()),
                audience_rating: audience_rating_pk
                    .and_then(|pk| lookups.audience_ratings.get(&pk).cloned()),
                language: language_pk.and_then(|pk| lookups.languages.get(&pk).cloned()),
                collection: collection_pk.and_then(|pk| lookups.collections.get(&pk).cloned()),
                rating: row.get(13)?,
                notes: row.get(14)?,
                plot: row.get(15)?,
                upc: row.get(16)?,
                front_cover: row.get(17)?,
                back_cover: row.get(18)?,
                release_year: row.get(19)?,
                date_added: row.get(20)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(movies)
}

/// Refuses to operate on what looks like a real, live Movie Collector
/// collection — only ever a test/working copy. Same hard rule as the
/// BookNojo/MusicNojo projects' tools.
pub fn ensure_not_real_collection(path: &Path) -> Result<()> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let text = canonical.to_string_lossy().to_lowercase();
    if text.contains("collectorz") && (text.contains("movie collector") || text.contains("movie_collector")) {
        anyhow::bail!(
            "{} looks like the real Movie Collector collection path \
             (~/Documents/Collectorz/...) — refused. Use a test copy only.",
            canonical.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_against_local_test_copy_if_present() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/MyCollection.mvcp");
        if !path.exists() {
            eprintln!("skipping: no local testdata/ (run refresh-testdata.sh)");
            return;
        }
        let conn = open_readonly(&path).unwrap();
        let movies = load_movies(&conn).unwrap();
        assert!(!movies.is_empty());
    }

    #[test]
    fn ensure_not_real_collection_matches_only_under_documents_collectorz() {
        assert!(ensure_not_real_collection(Path::new(
            "/Users/x/Documents/Collectorz/Movie Collector/MyCollection.mvcp"
        ))
        .is_err());
        assert!(ensure_not_real_collection(Path::new(
            "/Users/x/Developer/movie_collector_export/testdata/MyCollection.mvcp"
        ))
        .is_ok());
    }
}
