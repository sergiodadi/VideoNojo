use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::OptionalExtension;
pub use rusqlite::Connection;
use serde::{Deserialize, Serialize};

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
    pub role: Option<String>,
}

/// `ZCREW.ZROLEID`'s label, confirmed directly (role id 1 = Director,
/// 2 = Producer, 3 = Writer, 4 = Camera, 5 = Music, 6 = User Credit 1,
/// 7 = User Credit 2, 8 = Actor). No label exists for this anywhere in
/// `Movie_data` itself.
fn role_label(role_id: i64) -> Option<&'static str> {
    match role_id {
        1 => Some("Director"),
        2 => Some("Producer"),
        3 => Some("Writer"),
        4 => Some("Camera"),
        5 => Some("Music"),
        6 => Some("User Credit 1"),
        7 => Some("User Credit 2"),
        8 => Some("Actor"),
        _ => None,
    }
}

/// The inverse of `role_label`, case-insensitive — used by the write
/// path to turn a role name back into the id `ZCREW.ZROLEID` stores.
fn role_id_for_label(label: &str) -> Option<i64> {
    match label.to_lowercase().as_str() {
        "director" => Some(1),
        "producer" => Some(2),
        "writer" => Some(3),
        "camera" => Some(4),
        "music" => Some(5),
        "user credit 1" | "usercredit1" => Some(6),
        "user credit 2" | "usercredit2" => Some(7),
        "actor" => Some(8),
        _ => None,
    }
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
            role: role_id.and_then(role_label).map(str::to_string),
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

/// Opens a `.mvcp` package (or its `Movie_data` file directly) for
/// writing.
pub fn open_writable(db: &Path) -> Result<Connection> {
    let db_path = resolve_db_path(db);
    Connection::open(&db_path).with_context(|| format!("opening {}", db_path.display()))
}

/// Copies `Movie_data` to `backups/Movie_data.<nanos>.bak` before any
/// write — same convention as the other two projects.
pub fn backup_database(db: &Path) -> Result<()> {
    let db_path = resolve_db_path(db);
    let backups_dir = db_path
        .parent()
        .context("database path has no parent directory")?
        .join("backups");
    std::fs::create_dir_all(&backups_dir)?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let dest = backups_dir.join(format!("Movie_data.{nanos}.bak"));
    std::fs::copy(&db_path, &dest)
        .with_context(|| format!("backing up {} to {}", db_path.display(), dest.display()))?;
    Ok(())
}

/// The fields a new or edited movie can carry — deliberately a subset
/// of the full schema for now (see MOVIE.md for what's left).
#[derive(clap::Args, Deserialize, Default)]
pub struct MovieFields {
    /// Required to add a movie; omit on edit to leave the title
    /// unchanged.
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub title_original: Option<String>,
    #[arg(long)]
    pub release_year: Option<String>,
    #[arg(long)]
    pub rating: Option<i64>,
    #[arg(long)]
    pub notes: Option<String>,
    #[arg(long)]
    pub plot: Option<String>,
    #[arg(long)]
    pub upc: Option<String>,
    /// Must already exist (one of Movie Collector's own named
    /// sub-collections) — this never creates a new one.
    #[arg(long)]
    pub collection: Option<String>,
    #[arg(long)]
    pub country: Option<String>,
    #[arg(long = "genre")]
    pub genres: Vec<String>,
    #[arg(long = "studio")]
    pub studios: Vec<String>,
    #[arg(long = "format")]
    pub formats: Vec<String>,
    #[arg(long = "actor")]
    pub actors: Vec<String>,
    /// Crew names only for now — `ZROLEID` has no label anywhere in
    /// the database itself (see MOVIE.md § 4), so new crew rows are
    /// written with no role id rather than a guessed one.
    #[arg(long = "crew")]
    pub crew: Vec<String>,
}

/// Looks up the numeric Core Data entity id for an entity name, e.g.
/// `"Movie"` or `"LookupItemDelegate"`, directly from `Z_PRIMARYKEY`
/// rather than hardcoding every id this project's write path needs.
fn entity_id(conn: &Connection, name: &str) -> Result<i64> {
    conn.query_row("SELECT Z_ENT FROM Z_PRIMARYKEY WHERE Z_NAME = ?1", [name], |r| r.get(0))
        .with_context(|| format!("entity '{name}' not found in Z_PRIMARYKEY"))
}

/// Allocates new `Z_PK` values for a plan being built against a
/// read-only connection (nothing is actually written until `commit`
/// runs). Tracks how many pks it has already handed out per entity
/// this plan, so e.g. two new genres in the same `add` get distinct
/// pks instead of both reading the same stale `Z_MAX`.
struct PkAllocator<'a> {
    conn: &'a Connection,
    allocated: HashMap<String, i64>,
}

impl<'a> PkAllocator<'a> {
    fn new(conn: &'a Connection) -> Self {
        Self {
            conn,
            allocated: HashMap::new(),
        }
    }

    fn next(&mut self, entity_name: &str) -> Result<i64> {
        let base: i64 = self
            .conn
            .query_row("SELECT Z_MAX FROM Z_PRIMARYKEY WHERE Z_NAME = ?1", [entity_name], |r| {
                r.get(0)
            })
            .with_context(|| format!("entity '{entity_name}' not found in Z_PRIMARYKEY"))?;
        let count = self.allocated.entry(entity_name.to_string()).or_insert(0);
        *count += 1;
        Ok(base + *count)
    }

    /// One `UPDATE Z_PRIMARYKEY` statement per entity this plan
    /// actually allocated a new pk for.
    fn finish_sql(&self) -> Vec<String> {
        self.allocated
            .iter()
            .map(|(name, count)| {
                format!(
                    "UPDATE Z_PRIMARYKEY SET Z_MAX = Z_MAX + {count} WHERE Z_NAME = '{name}';"
                )
            })
            .collect()
    }
}

/// Core Data reference-date timestamp (seconds since 2001-01-01) for
/// "now" — same conversion as the read path's `ZDATEADDED` query, just
/// going the other direction.
fn current_core_data_timestamp() -> Result<f64> {
    let unix_seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs_f64();
    Ok(unix_seconds - 978_307_200.0)
}

fn sql_escape(s: &str) -> String {
    s.replace('\'', "''")
}

fn sql_literal(s: &Option<String>) -> String {
    match s {
        Some(v) => format!("'{}'", sql_escape(v)),
        None => "NULL".to_string(),
    }
}

/// Resolves `name` against an existing row of a two-hop lookup kind
/// (own thin table `table`, `ZDELEGATE → ZLOOKUPITEMDELEGATE`) by
/// exact display-name match, or stages the two INSERTs needed to
/// create one. Returns the lookup's own (not the delegate's) pk.
fn resolve_or_create_lookup(
    conn: &Connection,
    alloc: &mut PkAllocator,
    table: &str,
    table_entity: &str,
    name: &str,
    create_sql: &mut Vec<String>,
) -> Result<i64> {
    let existing: Option<i64> = conn
        .query_row(
            &format!(
                "SELECT t.Z_PK FROM {table} t \
                 JOIN ZLOOKUPITEMDELEGATE d ON t.ZDELEGATE = d.Z_PK \
                 WHERE d.ZDISPLAYNAME = ?1"
            ),
            [name],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(pk) = existing {
        return Ok(pk);
    }

    let delegate_pk = alloc.next("LookupItemDelegate")?;
    let delegate_ent = entity_id(conn, "LookupItemDelegate")?;
    let table_pk = alloc.next(table_entity)?;
    let table_ent = entity_id(conn, table_entity)?;
    let escaped = sql_escape(name);

    create_sql.push(format!(
        "INSERT INTO ZLOOKUPITEMDELEGATE (Z_PK, Z_ENT, Z_OPT, ZDISPLAYNAME, ZSORTNAME) \
         VALUES ({delegate_pk}, {delegate_ent}, 1, '{escaped}', '{escaped}');"
    ));
    create_sql.push(format!(
        "INSERT INTO {table} (Z_PK, Z_ENT, Z_OPT, ZDELEGATE) VALUES ({table_pk}, {table_ent}, 1, {delegate_pk});"
    ));
    Ok(table_pk)
}

/// Same shape as `resolve_or_create_lookup`, but via `ZPERSONDELEGATE`
/// for person-like entities (`Actor`, `CreditPerson`).
fn resolve_or_create_person(
    conn: &Connection,
    alloc: &mut PkAllocator,
    table: &str,
    table_entity: &str,
    name: &str,
    create_sql: &mut Vec<String>,
) -> Result<i64> {
    let existing: Option<i64> = conn
        .query_row(
            &format!(
                "SELECT t.Z_PK FROM {table} t \
                 JOIN ZPERSONDELEGATE d ON t.ZDELEGATE = d.Z_PK \
                 WHERE d.ZDISPLAYNAME = ?1"
            ),
            [name],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(pk) = existing {
        return Ok(pk);
    }

    let delegate_pk = alloc.next("PersonDelegate")?;
    let delegate_ent = entity_id(conn, "PersonDelegate")?;
    let table_pk = alloc.next(table_entity)?;
    let table_ent = entity_id(conn, table_entity)?;
    let escaped = sql_escape(name);

    create_sql.push(format!(
        "INSERT INTO ZPERSONDELEGATE (Z_PK, Z_ENT, Z_OPT, ZDISPLAYNAME, ZSORTNAME) \
         VALUES ({delegate_pk}, {delegate_ent}, 1, '{escaped}', '{escaped}');"
    ));
    create_sql.push(format!(
        "INSERT INTO {table} (Z_PK, Z_ENT, Z_OPT, ZDELEGATE) VALUES ({table_pk}, {table_ent}, 1, {delegate_pk});"
    ));
    Ok(table_pk)
}

/// Resolves an existing sub-collection by exact title match — never
/// creates one (same rule as Book/Music Collector's own
/// `AlbumFields.collection`/`BookFields`-equivalent).
fn resolve_collection_pk(conn: &Connection, name: &str) -> Result<i64> {
    conn.query_row("SELECT Z_PK FROM ZCOLLECTION WHERE ZTITLE = ?1", [name], |r| r.get(0))
        .with_context(|| format!("no sub-collection named '{name}'"))
}

/// Builds the full SQL text to add one movie, wrapped in
/// `BEGIN;`/`COMMIT;` — resolved against `conn` (a read-only
/// connection is fine, nothing here is executed) but never run by
/// this function itself. `commit_plan` executes the result later.
pub fn build_plan(conn: &Connection, fields: &MovieFields) -> Result<String> {
    let title = fields.title.clone().context("title is required to add a movie")?;
    let mut alloc = PkAllocator::new(conn);
    let mut create_sql = Vec::new();

    let country_pk = fields
        .country
        .as_deref()
        .map(|name| resolve_or_create_lookup(conn, &mut alloc, "ZCOUNTRY", "Country", name, &mut create_sql))
        .transpose()?;
    let collection_pk = fields
        .collection
        .as_deref()
        .map(|name| resolve_collection_pk(conn, name))
        .transpose()?;

    let genre_pks = fields
        .genres
        .iter()
        .map(|name| resolve_or_create_lookup(conn, &mut alloc, "ZGENRE", "Genre", name, &mut create_sql))
        .collect::<Result<Vec<_>>>()?;
    let studio_pks = fields
        .studios
        .iter()
        .map(|name| resolve_or_create_lookup(conn, &mut alloc, "ZSTUDIO", "Studio", name, &mut create_sql))
        .collect::<Result<Vec<_>>>()?;
    let format_pks = fields
        .formats
        .iter()
        .map(|name| resolve_or_create_lookup(conn, &mut alloc, "ZFORMAT", "Format", name, &mut create_sql))
        .collect::<Result<Vec<_>>>()?;
    let actor_pks = fields
        .actors
        .iter()
        .map(|name| resolve_or_create_person(conn, &mut alloc, "ZACTOR", "Actor", name, &mut create_sql))
        .collect::<Result<Vec<_>>>()?;
    // Each entry is "Name" or "Name:Role" (e.g. "Jane Doe:Director") —
    // the role is optional since most crew credits in practice don't
    // need one beyond the person's name.
    let crew_entries: Vec<(&str, Option<i64>)> = fields
        .crew
        .iter()
        .map(|entry| match entry.split_once(':') {
            Some((name, role)) => (name, role_id_for_label(role)),
            None => (entry.as_str(), None),
        })
        .collect();
    let crew_pks = crew_entries
        .iter()
        .map(|(name, _)| resolve_or_create_person(conn, &mut alloc, "ZCREDITPERSON", "CreditPerson", name, &mut create_sql))
        .collect::<Result<Vec<_>>>()?;

    let movie_pk = alloc.next("Collectible")?;

    let mut statements = Vec::new();
    statements.push("BEGIN;".to_string());
    statements.extend(create_sql);
    statements.push(format!(
        "INSERT INTO ZCOLLECTIBLE (Z_PK, Z_ENT, Z_OPT, ZTITLE, ZTITLEORIGINAL, ZCOUNTRY, \
         ZCOLLECTION, ZRELEASEYEAR, ZMYRATING, ZNOTES, ZPLOT, ZUPC, ZDATEADDED) \
         VALUES ({movie_pk}, {ent_movie}, 1, '{title}', {title_original}, {country}, \
         {collection}, {release_year}, {rating}, {notes}, {plot}, {upc}, {date_added});",
        ent_movie = ENT_MOVIE,
        title = sql_escape(&title),
        title_original = sql_literal(&fields.title_original),
        country = country_pk.map(|p| p.to_string()).unwrap_or_else(|| "NULL".to_string()),
        collection = collection_pk.map(|p| p.to_string()).unwrap_or_else(|| "NULL".to_string()),
        release_year = sql_literal(&fields.release_year),
        rating = fields.rating.map(|r| r.to_string()).unwrap_or_else(|| "NULL".to_string()),
        notes = sql_literal(&fields.notes),
        plot = sql_literal(&fields.plot),
        upc = sql_literal(&fields.upc),
        date_added = current_core_data_timestamp()?,
    ));

    // The many-to-many join tables have no Z_PK of their own (just the
    // two FK columns) — a plain INSERT of the pair, no allocation.
    for pk in &genre_pks {
        statements.push(format!(
            "INSERT INTO Z_20MOVIES (Z_20GENRES, Z_7MOVIES3) VALUES ({pk}, {movie_pk});"
        ));
    }
    for pk in &studio_pks {
        statements.push(format!(
            "INSERT INTO Z_7STUDIOS (Z_41STUDIOS, Z_7MOVIES7) VALUES ({pk}, {movie_pk});"
        ));
    }
    for pk in &format_pks {
        statements.push(format!(
            "INSERT INTO Z_19MOVIES (Z_19FORMATS, Z_7MOVIES2) VALUES ({pk}, {movie_pk});"
        ));
    }

    let cast_ent = entity_id(conn, "Cast")?;
    for (rank, pk) in actor_pks.iter().enumerate() {
        let cast_pk = alloc.next("Cast")?;
        statements.push(format!(
            "INSERT INTO ZCAST (Z_PK, Z_ENT, Z_OPT, ZMOVIE, ZACTOR, ZRANK) \
             VALUES ({cast_pk}, {cast_ent}, 1, {movie_pk}, {pk}, {rank});"
        ));
    }

    let crew_ent = entity_id(conn, "Crew")?;
    for (rank, pk) in crew_pks.iter().enumerate() {
        let crew_pk = alloc.next("Crew")?;
        let role_id = crew_entries[rank].1;
        let role_sql = role_id.map(|r| r.to_string()).unwrap_or_else(|| "NULL".to_string());
        statements.push(format!(
            "INSERT INTO ZCREW (Z_PK, Z_ENT, Z_OPT, ZMOVIE, ZCREDITPERSON, ZRANK, ZROLEID) \
             VALUES ({crew_pk}, {crew_ent}, 1, {movie_pk}, {pk}, {rank}, {role_sql});"
        ));
    }

    statements.extend(alloc.finish_sql());
    statements.push("COMMIT;".to_string());

    Ok(statements.join("\n"))
}

/// Executes a plan produced by `build_plan` (or any `BEGIN;`/`COMMIT;`
/// wrapped SQL text) against a writable connection, in one
/// transaction (SQLite treats a script's own `BEGIN`/`COMMIT` as the
/// transaction boundary here, so no extra wrapping is needed).
pub fn commit_plan(conn: &Connection, sql: &str) -> Result<()> {
    conn.execute_batch(sql)?;
    Ok(())
}

/// `clap` can't tell "flag omitted" from "list given zero times" on a
/// repeatable `Vec<String>` — these flags opt an edit into replacing
/// the corresponding list rather than leaving it untouched, same
/// convention as Book/Music Collector's own edit commands.
#[derive(clap::Args, Deserialize, Default)]
pub struct ReplaceFlags {
    #[arg(long)]
    pub replace_genres: bool,
    #[arg(long)]
    pub replace_studios: bool,
    #[arg(long)]
    pub replace_formats: bool,
    #[arg(long)]
    pub replace_actors: bool,
    #[arg(long)]
    pub replace_crew: bool,
}

/// Builds the SQL text to update an existing movie's fields — any
/// `MovieFields` left `None`/empty is left untouched, except a
/// many-to-many/role list explicitly opted into replacement via
/// `replace`. Never creates a new movie, never touches a relation
/// this project doesn't itself manage (see MOVIE.md's own note on
/// `delete`'s scope — the same limitation applies here: edit can
/// replace what it created, not relations like discs/links/tags that
/// aren't modeled yet).
pub fn build_edit_plan(conn: &Connection, id: i64, fields: &MovieFields, replace: &ReplaceFlags) -> Result<String> {
    let mut alloc = PkAllocator::new(conn);
    let mut statements = vec!["BEGIN;".to_string()];

    let mut set_clauses = Vec::new();
    if let Some(title) = &fields.title {
        set_clauses.push(format!("ZTITLE = '{}'", sql_escape(title)));
    }
    if let Some(v) = &fields.title_original {
        set_clauses.push(format!("ZTITLEORIGINAL = '{}'", sql_escape(v)));
    }
    if let Some(v) = &fields.release_year {
        set_clauses.push(format!("ZRELEASEYEAR = '{}'", sql_escape(v)));
    }
    if let Some(v) = fields.rating {
        set_clauses.push(format!("ZMYRATING = {v}"));
    }
    if let Some(v) = &fields.notes {
        set_clauses.push(format!("ZNOTES = '{}'", sql_escape(v)));
    }
    if let Some(v) = &fields.plot {
        set_clauses.push(format!("ZPLOT = '{}'", sql_escape(v)));
    }
    if let Some(v) = &fields.upc {
        set_clauses.push(format!("ZUPC = '{}'", sql_escape(v)));
    }
    if let Some(name) = &fields.country {
        let pk = resolve_or_create_lookup(conn, &mut alloc, "ZCOUNTRY", "Country", name, &mut statements)?;
        set_clauses.push(format!("ZCOUNTRY = {pk}"));
    }
    if let Some(name) = &fields.collection {
        let pk = resolve_collection_pk(conn, name)?;
        set_clauses.push(format!("ZCOLLECTION = {pk}"));
    }
    if !set_clauses.is_empty() {
        statements.push(format!(
            "UPDATE ZCOLLECTIBLE SET {} WHERE Z_PK = {id};",
            set_clauses.join(", ")
        ));
    }

    if replace.replace_genres {
        statements.push(format!("DELETE FROM Z_20MOVIES WHERE Z_7MOVIES3 = {id};"));
        for name in &fields.genres {
            let pk = resolve_or_create_lookup(conn, &mut alloc, "ZGENRE", "Genre", name, &mut statements)?;
            statements.push(format!("INSERT INTO Z_20MOVIES (Z_20GENRES, Z_7MOVIES3) VALUES ({pk}, {id});"));
        }
    }
    if replace.replace_studios {
        statements.push(format!("DELETE FROM Z_7STUDIOS WHERE Z_7MOVIES7 = {id};"));
        for name in &fields.studios {
            let pk = resolve_or_create_lookup(conn, &mut alloc, "ZSTUDIO", "Studio", name, &mut statements)?;
            statements.push(format!("INSERT INTO Z_7STUDIOS (Z_41STUDIOS, Z_7MOVIES7) VALUES ({pk}, {id});"));
        }
    }
    if replace.replace_formats {
        statements.push(format!("DELETE FROM Z_19MOVIES WHERE Z_7MOVIES2 = {id};"));
        for name in &fields.formats {
            let pk = resolve_or_create_lookup(conn, &mut alloc, "ZFORMAT", "Format", name, &mut statements)?;
            statements.push(format!("INSERT INTO Z_19MOVIES (Z_19FORMATS, Z_7MOVIES2) VALUES ({pk}, {id});"));
        }
    }
    if replace.replace_actors {
        statements.push(format!("DELETE FROM ZCAST WHERE ZMOVIE = {id};"));
        let cast_ent = entity_id(conn, "Cast")?;
        for (rank, name) in fields.actors.iter().enumerate() {
            let pk = resolve_or_create_person(conn, &mut alloc, "ZACTOR", "Actor", name, &mut statements)?;
            let cast_pk = alloc.next("Cast")?;
            statements.push(format!(
                "INSERT INTO ZCAST (Z_PK, Z_ENT, Z_OPT, ZMOVIE, ZACTOR, ZRANK) VALUES ({cast_pk}, {cast_ent}, 1, {id}, {pk}, {rank});"
            ));
        }
    }
    if replace.replace_crew {
        statements.push(format!("DELETE FROM ZCREW WHERE ZMOVIE = {id};"));
        let crew_ent = entity_id(conn, "Crew")?;
        for (rank, entry) in fields.crew.iter().enumerate() {
            let (name, role_id) = match entry.split_once(':') {
                Some((name, role)) => (name, role_id_for_label(role)),
                None => (entry.as_str(), None),
            };
            let pk = resolve_or_create_person(conn, &mut alloc, "ZCREDITPERSON", "CreditPerson", name, &mut statements)?;
            let crew_pk = alloc.next("Crew")?;
            let role_sql = role_id.map(|r| r.to_string()).unwrap_or_else(|| "NULL".to_string());
            statements.push(format!(
                "INSERT INTO ZCREW (Z_PK, Z_ENT, Z_OPT, ZMOVIE, ZCREDITPERSON, ZRANK, ZROLEID) VALUES ({crew_pk}, {crew_ent}, 1, {id}, {pk}, {rank}, {role_sql});"
            ));
        }
    }

    statements.extend(alloc.finish_sql());
    statements.push("COMMIT;".to_string());
    Ok(statements.join("\n"))
}

/// Builds the SQL text to delete a movie. Removes the movie's own row
/// plus every relation this project's own write path creates (cast,
/// crew, genre/studio/format links) — **not yet** the relations
/// nothing here models yet (discs, episodes, links, tags, region/
/// screen-ratio/subtitle/HDR/extra/audio-track links), same
/// deliberately-documented scope limit as Book/Music Collector's own
/// delete left for fields they hadn't modeled either. Shared lookup/
/// person values (a genre, a studio, an actor) are never deleted,
/// even if this was their only reference — same conservative,
/// reference-counted-style choice as the other two projects.
pub fn build_delete_plan(id: i64) -> String {
    vec![
        "BEGIN;".to_string(),
        format!("DELETE FROM ZCAST WHERE ZMOVIE = {id};"),
        format!("DELETE FROM ZCREW WHERE ZMOVIE = {id};"),
        format!("DELETE FROM Z_20MOVIES WHERE Z_7MOVIES3 = {id};"),
        format!("DELETE FROM Z_7STUDIOS WHERE Z_7MOVIES7 = {id};"),
        format!("DELETE FROM Z_19MOVIES WHERE Z_7MOVIES2 = {id};"),
        format!("DELETE FROM ZCOLLECTIBLE WHERE Z_PK = {id};"),
        "COMMIT;".to_string(),
    ]
    .join("\n")
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

    /// A fresh, disposable, writable copy of `testdata/` in a new temp
    /// dir, or `None` if `testdata/` isn't present locally (run
    /// `refresh-testdata.sh`). Caller is responsible for removing the
    /// returned scratch directory when done.
    fn fresh_scratch_copy() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/MyCollection.mvcp");
        if !source.exists() {
            eprintln!("skipping: no local testdata/ (run refresh-testdata.sh)");
            return None;
        }
        let scratch = std::env::temp_dir().join(format!(
            "vc_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let dest = scratch.join("MyCollection.mvcp");
        std::fs::create_dir_all(&scratch).unwrap();
        copy_dir(&source, &dest).unwrap();
        // The source copy is read-only (chmod'd by refresh-testdata.sh)
        // — make this disposable copy writable before using it.
        for entry in std::fs::read_dir(&dest).unwrap() {
            let entry = entry.unwrap();
            let mut perms = entry.metadata().unwrap().permissions();
            perms.set_readonly(false);
            std::fs::set_permissions(entry.path(), perms).unwrap();
        }
        Some((scratch, dest))
    }

    #[test]
    fn add_movie_round_trip_through_every_relation() {
        let Some((scratch, dest)) = fresh_scratch_copy() else {
            return;
        };

        let fields = MovieFields {
            title: Some("Round Trip Test Movie".to_string()),
            release_year: Some("1999".to_string()),
            rating: Some(5),
            country: Some("Round Trip Testland".to_string()),
            genres: vec!["Round Trip Genre".to_string()],
            studios: vec!["Round Trip Studio".to_string()],
            actors: vec!["Round Trip Actor".to_string()],
            crew: vec!["Round Trip Crew:Director".to_string()],
            ..Default::default()
        };

        let plan = {
            let conn = open_readonly(&dest).unwrap();
            build_plan(&conn, &fields).unwrap()
        };
        {
            let conn = open_writable(&dest).unwrap();
            commit_plan(&conn, &plan).unwrap();
        }

        let conn = open_readonly(&dest).unwrap();
        let movies = load_movies(&conn).unwrap();
        let added = movies
            .iter()
            .find(|m| m.title == "Round Trip Test Movie")
            .expect("newly added movie not found");
        assert_eq!(added.country.as_deref(), Some("Round Trip Testland"));
        assert_eq!(added.genres, vec!["Round Trip Genre".to_string()]);
        assert_eq!(added.studios, vec!["Round Trip Studio".to_string()]);
        assert_eq!(added.cast.len(), 1);
        assert_eq!(added.cast[0].name, "Round Trip Actor");
        assert_eq!(added.crew.len(), 1);
        assert_eq!(added.crew[0].name, "Round Trip Crew");
        assert_eq!(added.crew[0].role.as_deref(), Some("Director"));

        drop(conn);
        std::fs::remove_dir_all(&scratch).ok();
    }

    #[test]
    fn edit_movie_updates_fields_and_can_replace_lists() {
        let Some((scratch, dest)) = fresh_scratch_copy() else {
            return;
        };

        let add_fields = MovieFields {
            title: Some("Edit Target Movie".to_string()),
            genres: vec!["Edit Original Genre".to_string()],
            ..Default::default()
        };
        let id = {
            let conn = open_readonly(&dest).unwrap();
            let plan = build_plan(&conn, &add_fields).unwrap();
            drop(conn);
            let conn = open_writable(&dest).unwrap();
            commit_plan(&conn, &plan).unwrap();
            drop(conn);
            let conn = open_readonly(&dest).unwrap();
            load_movies(&conn)
                .unwrap()
                .into_iter()
                .find(|m| m.title == "Edit Target Movie")
                .unwrap()
                .id
        };

        let edit_fields = MovieFields {
            title: Some("Edit Target Movie (renamed)".to_string()),
            rating: Some(3),
            genres: vec!["Edit Replacement Genre".to_string()],
            ..Default::default()
        };
        let replace = ReplaceFlags {
            replace_genres: true,
            ..Default::default()
        };
        {
            let conn = open_readonly(&dest).unwrap();
            let plan = build_edit_plan(&conn, id, &edit_fields, &replace).unwrap();
            drop(conn);
            let conn = open_writable(&dest).unwrap();
            commit_plan(&conn, &plan).unwrap();
        }

        let conn = open_readonly(&dest).unwrap();
        let movies = load_movies(&conn).unwrap();
        let edited = movies.iter().find(|m| m.id == id).unwrap();
        assert_eq!(edited.title, "Edit Target Movie (renamed)");
        assert_eq!(edited.rating, Some(3));
        assert_eq!(edited.genres, vec!["Edit Replacement Genre".to_string()]);

        drop(conn);
        std::fs::remove_dir_all(&scratch).ok();
    }

    #[test]
    fn delete_movie_removes_only_its_own_rows() {
        let Some((scratch, dest)) = fresh_scratch_copy() else {
            return;
        };

        // Two movies sharing one genre — deleting one must not delete
        // the shared genre value or the other movie.
        let shared_genre = "Delete Test Shared Genre".to_string();
        let (keep_id, delete_id) = {
            let conn = open_readonly(&dest).unwrap();
            let plan_a = build_plan(
                &conn,
                &MovieFields {
                    title: Some("Delete Test Keep".to_string()),
                    genres: vec![shared_genre.clone()],
                    ..Default::default()
                },
            )
            .unwrap();
            drop(conn);
            let conn = open_writable(&dest).unwrap();
            commit_plan(&conn, &plan_a).unwrap();
            drop(conn);

            let conn = open_readonly(&dest).unwrap();
            let plan_b = build_plan(
                &conn,
                &MovieFields {
                    title: Some("Delete Test Remove".to_string()),
                    genres: vec![shared_genre.clone()],
                    actors: vec!["Delete Test Actor".to_string()],
                    ..Default::default()
                },
            )
            .unwrap();
            drop(conn);
            let conn = open_writable(&dest).unwrap();
            commit_plan(&conn, &plan_b).unwrap();
            drop(conn);

            let conn = open_readonly(&dest).unwrap();
            let movies = load_movies(&conn).unwrap();
            let keep = movies.iter().find(|m| m.title == "Delete Test Keep").unwrap().id;
            let remove = movies.iter().find(|m| m.title == "Delete Test Remove").unwrap().id;
            (keep, remove)
        };

        {
            let plan = build_delete_plan(delete_id);
            let conn = open_writable(&dest).unwrap();
            commit_plan(&conn, &plan).unwrap();
        }

        let conn = open_readonly(&dest).unwrap();
        let movies = load_movies(&conn).unwrap();
        assert!(!movies.iter().any(|m| m.id == delete_id));
        let kept = movies.iter().find(|m| m.id == keep_id).unwrap();
        assert_eq!(kept.genres, vec![shared_genre]);

        drop(conn);
        std::fs::remove_dir_all(&scratch).ok();
    }

    fn copy_dir(source: &Path, dest: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dest)?;
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            let dest_path = dest.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_dir(&entry.path(), &dest_path)?;
            } else {
                std::fs::copy(entry.path(), &dest_path)?;
            }
        }
        Ok(())
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
