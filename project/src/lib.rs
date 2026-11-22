use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

// `ZLINK` reuses one shared table for two sibling entities under the
// abstract "Link" root (`Z_ENT = 25`, never stored directly) — same
// numbered-duplicate-column pattern as Music Collector's own
// `ZDISC`/`ZDISC1`. A `LocalLink` row populates `ZITEMDELEGATE`/
// `ZMOVIE`; an `OnlineLink` row populates `ZITEMDELEGATE1`/`ZMOVIE1`
// instead — confirmed directly (every real link in the test
// collection is `OnlineLink`, all with `ZMOVIE` null/`ZMOVIE1` set).
const ENT_LOCAL_LINK: i64 = 26;
const ENT_ONLINE_LINK: i64 = 27;

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

/// A TV episode, one row of `ZEPISODE` — always a child of a `Disc`
/// (`ZEPISODE.ZDISC`), never directly of a `Movie`. Only the
/// columns with real data in the test collection are modeled; a
/// handful of sparser ones (IMDB number, movie URL, viewing
/// date/location) are left out, same "deliberately a subset" scope
/// already noted for `MovieFields`.
#[derive(Serialize, Clone)]
pub struct Episode {
    pub id: i64,
    pub rank: Option<i64>,
    pub sequence_number: Option<String>,
    pub title: Option<String>,
    pub plot: Option<String>,
    pub running_time: Option<i64>,
    pub seen_it: bool,
    pub first_air_year: Option<String>,
    pub first_air_month: Option<String>,
    pub first_air_day: Option<String>,
}

/// A physical disc, one row of `ZDISC` — its own top-level entity
/// (`ZDISC.ZMOVIE`), not part of the `Collectible` hierarchy. Holds
/// zero or more `Episode`s for a TV-series movie.
#[derive(Serialize, Clone)]
pub struct Disc {
    pub id: i64,
    pub rank: Option<i64>,
    pub title: Option<String>,
    pub format: Option<String>,
    pub storage_device: Option<String>,
    pub extra_features: Option<String>,
    pub storage_slot: Option<String>,
    pub episodes: Vec<Episode>,
}

/// A local file reference or an online URL (IMDB, trailer, review
/// site...) attached to a movie, one row of `ZLINK`. Read-only for
/// now — only `OnlineLink` has any real data to verify a write shape
/// against (see `ENT_LOCAL_LINK`'s own doc comment).
#[derive(Serialize, Clone)]
pub struct Link {
    pub id: i64,
    /// `"local"` or `"online"`.
    pub kind: &'static str,
    pub description: Option<String>,
    pub url: Option<String>,
    pub rank: Option<i64>,
}

#[derive(Serialize)]
pub struct Movie {
    pub id: i64,
    pub title: String,
    pub title_original: Option<String>,
    pub cast: Vec<CastMember>,
    pub crew: Vec<Credit>,
    pub discs: Vec<Disc>,
    pub links: Vec<Link>,
    pub genres: Vec<String>,
    pub studios: Vec<String>,
    pub formats: Vec<String>,
    pub regions: Vec<String>,
    pub screen_ratios: Vec<String>,
    pub subtitles: Vec<String>,
    pub audio_tracks: Vec<String>,
    pub hdr: Vec<String>,
    pub extras: Vec<String>,
    pub tags: Vec<String>,
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
    pub series: Option<String>,
    pub box_set: Option<String>,
    pub box_set_rank: Option<i64>,
    pub is_tv_series: bool,
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
    series: HashMap<i64, String>,
    box_sets: HashMap<i64, String>,
    // Disc-level lookups, keyed by the lookup's own Z_PK (not by
    // movie id like `formats` above, which is a movie→format
    // many-to-many map) — `ZDISC.ZFORMAT`/`ZSTORAGEDEVICE` are
    // single-valued FKs straight off the disc row.
    disc_formats: HashMap<i64, String>,
    storage_devices: HashMap<i64, String>,
    actors: HashMap<i64, String>,
    credit_people: HashMap<i64, String>,
    genres: HashMap<i64, Vec<String>>,
    studios: HashMap<i64, Vec<String>>,
    formats: HashMap<i64, Vec<String>>,
    regions: HashMap<i64, Vec<String>>,
    screen_ratios: HashMap<i64, Vec<String>>,
    subtitles: HashMap<i64, Vec<String>>,
    audio_tracks: HashMap<i64, Vec<String>>,
    hdr: HashMap<i64, Vec<String>>,
    extras: HashMap<i64, Vec<String>>,
    tags: HashMap<i64, Vec<String>>,
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
        series: load_lookup_map(conn, "ZSERIES")?,
        box_sets: load_lookup_map(conn, "ZBOXSET")?,
        disc_formats: load_lookup_map(conn, "ZFORMAT")?,
        storage_devices: load_lookup_map(conn, "ZSTORAGEDEVICE")?,
        collections: load_collections_map(conn)?,
        actors: load_person_map(conn, "ZACTOR")?,
        credit_people: load_person_map(conn, "ZCREDITPERSON")?,
        genres: load_many_to_many(conn, "Z_20MOVIES", "Z_7MOVIES3", "Z_20GENRES", "ZGENRE")?,
        studios: load_many_to_many(conn, "Z_7STUDIOS", "Z_7MOVIES7", "Z_41STUDIOS", "ZSTUDIO")?,
        formats: load_many_to_many(conn, "Z_19MOVIES", "Z_7MOVIES2", "Z_19FORMATS", "ZFORMAT")?,
        regions: load_many_to_many(conn, "Z_7REGIONS", "Z_7MOVIES8", "Z_35REGIONS", "ZREGION")?,
        screen_ratios: load_many_to_many(
            conn,
            "Z_7SCREENRATIOS",
            "Z_7MOVIES6",
            "Z_36SCREENRATIOS",
            "ZSCREENRATIO",
        )?,
        subtitles: load_many_to_many(conn, "Z_7SUBTITLES", "Z_7MOVIES5", "Z_42SUBTITLES", "ZSUBTITLE")?,
        audio_tracks: load_many_to_many(conn, "Z_3MOVIES", "Z_7MOVIES", "Z_3AUDIOTRACKS", "ZAUDIOTRACK")?,
        hdr: load_many_to_many(conn, "Z_21MOVIES", "Z_7MOVIES4", "Z_21HDRS", "ZHDR")?,
        extras: load_many_to_many(conn, "Z_17MOVIES", "Z_7MOVIES1", "Z_17EXTRAS", "ZEXTRA")?,
        tags: load_many_to_many(conn, "Z_6TAGS", "Z_6COLLECTIBLES1", "Z_43TAGS", "ZTAG")?,
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

/// `ZEPISODE` rows, grouped by their owning disc's `Z_PK`
/// (`ZEPISODE.ZDISC`) — an episode never links to a movie directly.
fn load_episodes(conn: &Connection) -> Result<HashMap<i64, Vec<Episode>>> {
    let mut stmt = conn.prepare(
        "SELECT Z_PK, ZDISC, ZRANK, ZSEQUENCENUMBER, ZTITLE, ZPLOT, ZRUNNINGTIME, \
                ZSEENIT, ZFIRSTAIRYEAR, ZFIRSTAIRMONTH, ZFIRSTAIRDAY \
         FROM ZEPISODE ORDER BY ZDISC, ZRANK",
    )?;
    let mut map: HashMap<i64, Vec<Episode>> = HashMap::new();
    let rows = stmt.query_map([], |row| {
        let disc_pk: Option<i64> = row.get(1)?;
        Ok((
            disc_pk,
            Episode {
                id: row.get(0)?,
                rank: row.get(2)?,
                sequence_number: row.get(3)?,
                title: row.get(4)?,
                plot: row.get(5)?,
                running_time: row.get(6)?,
                seen_it: row.get::<_, Option<i64>>(7)?.unwrap_or(0) != 0,
                first_air_year: row.get(8)?,
                first_air_month: row.get(9)?,
                first_air_day: row.get(10)?,
            },
        ))
    })?;
    for (disc_pk, episode) in rows.filter_map(|r| r.ok()) {
        let Some(disc_pk) = disc_pk else { continue };
        map.entry(disc_pk).or_default().push(episode);
    }
    Ok(map)
}

/// `ZDISC` rows, grouped by their owning movie's `Z_PK`
/// (`ZDISC.ZMOVIE`), each with its own `Episode`s already attached.
fn load_discs(conn: &Connection, lookups: &Lookups) -> Result<HashMap<i64, Vec<Disc>>> {
    let episodes_by_disc = load_episodes(conn)?;
    let mut stmt = conn.prepare(
        "SELECT Z_PK, ZMOVIE, ZRANK, ZTITLE, ZFORMAT, ZSTORAGEDEVICE, ZEXTRAFEATURES, ZSTORAGESLOT \
         FROM ZDISC ORDER BY ZMOVIE, ZRANK",
    )?;
    let mut map: HashMap<i64, Vec<Disc>> = HashMap::new();
    let rows = stmt.query_map([], |row| {
        let movie_pk: Option<i64> = row.get(1)?;
        let id: i64 = row.get(0)?;
        let format_pk: Option<i64> = row.get(4)?;
        let storage_device_pk: Option<i64> = row.get(5)?;
        Ok((
            movie_pk,
            Disc {
                id,
                rank: row.get(2)?,
                title: row.get(3)?,
                format: format_pk.and_then(|pk| lookups.disc_formats.get(&pk).cloned()),
                storage_device: storage_device_pk.and_then(|pk| lookups.storage_devices.get(&pk).cloned()),
                extra_features: row.get(6)?,
                storage_slot: row.get(7)?,
                episodes: episodes_by_disc.get(&id).cloned().unwrap_or_default(),
            },
        ))
    })?;
    for (movie_pk, disc) in rows.filter_map(|r| r.ok()) {
        let Some(movie_pk) = movie_pk else { continue };
        map.entry(movie_pk).or_default().push(disc);
    }
    Ok(map)
}

/// `ZLINK` rows, grouped by movie — reads whichever numbered pair
/// (`ZMOVIE`/`ZITEMDELEGATE` or `ZMOVIE1`/`ZITEMDELEGATE1`) the row's
/// own `Z_ENT` says is the live one, rather than assuming the
/// unsuffixed column is always populated (see `ENT_LOCAL_LINK`'s own
/// doc comment).
fn load_links(conn: &Connection) -> Result<HashMap<i64, Vec<Link>>> {
    let mut stmt = conn.prepare(
        "SELECT Z_PK, Z_ENT, ZRANK, ZDESCRIPTION, ZURL, ZMOVIE, ZMOVIE1 FROM ZLINK \
         ORDER BY COALESCE(ZMOVIE, ZMOVIE1), ZRANK",
    )?;
    let mut map: HashMap<i64, Vec<Link>> = HashMap::new();
    let rows = stmt.query_map([], |row| {
        let ent: i64 = row.get(1)?;
        let movie_pk: Option<i64> = if ent == ENT_LOCAL_LINK { row.get(5)? } else { row.get(6)? };
        Ok((
            ent,
            movie_pk,
            Link {
                id: row.get(0)?,
                kind: if ent == ENT_LOCAL_LINK { "local" } else { "online" },
                description: row.get(3)?,
                url: row.get(4)?,
                rank: row.get(2)?,
            },
        ))
    })?;
    for (ent, movie_pk, link) in rows.filter_map(|r| r.ok()) {
        // Only Link's two known children — guards against some other
        // schema variant silently landing in the wrong bucket.
        if ent != ENT_LOCAL_LINK && ent != ENT_ONLINE_LINK {
            continue;
        }
        let Some(movie_pk) = movie_pk else { continue };
        map.entry(movie_pk).or_default().push(link);
    }
    Ok(map)
}

pub fn load_movies(conn: &Connection) -> Result<Vec<Movie>> {
    let lookups = load_lookups(conn)?;
    let cast_by_movie = load_cast(conn, &lookups)?;
    let crew_by_movie = load_crew(conn, &lookups)?;
    let discs_by_movie = load_discs(conn, &lookups)?;
    let links_by_movie = load_links(conn)?;

    let mut stmt = conn.prepare(
        "SELECT Z_PK, ZTITLE, ZTITLEORIGINAL, ZCOUNTRY, ZCONDITION, ZPACKAGING, \
                ZDISTRIBUTOR, ZEDITION, ZLOCATION, ZOWNER, ZAUDIENCERATING, \
                ZLANGUAGE, ZCOLLECTION, ZMYRATING, ZNOTES, ZPLOT, ZUPC, \
                ZFRONTCOVER, ZBACKCOVER, ZRELEASEYEAR, \
                datetime(ZDATEADDED + 978307200, 'unixepoch'), \
                ZSERIES, ZBOXSET, ZBOXSETRANK, ZISTVSERIES \
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
            let series_pk: Option<i64> = row.get(21)?;
            let box_set_pk: Option<i64> = row.get(22)?;

            Ok(Movie {
                id,
                title: row.get(1)?,
                title_original: row.get(2)?,
                cast: cast_by_movie.get(&id).cloned().unwrap_or_default(),
                crew: crew_by_movie.get(&id).cloned().unwrap_or_default(),
                discs: discs_by_movie.get(&id).cloned().unwrap_or_default(),
                links: links_by_movie.get(&id).cloned().unwrap_or_default(),
                genres: lookups.genres.get(&id).cloned().unwrap_or_default(),
                studios: lookups.studios.get(&id).cloned().unwrap_or_default(),
                formats: lookups.formats.get(&id).cloned().unwrap_or_default(),
                regions: lookups.regions.get(&id).cloned().unwrap_or_default(),
                screen_ratios: lookups.screen_ratios.get(&id).cloned().unwrap_or_default(),
                subtitles: lookups.subtitles.get(&id).cloned().unwrap_or_default(),
                audio_tracks: lookups.audio_tracks.get(&id).cloned().unwrap_or_default(),
                hdr: lookups.hdr.get(&id).cloned().unwrap_or_default(),
                extras: lookups.extras.get(&id).cloned().unwrap_or_default(),
                tags: lookups.tags.get(&id).cloned().unwrap_or_default(),
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
                series: series_pk.and_then(|pk| lookups.series.get(&pk).cloned()),
                box_set: box_set_pk.and_then(|pk| lookups.box_sets.get(&pk).cloned()),
                box_set_rank: row.get(23)?,
                is_tv_series: row.get::<_, Option<i64>>(24)?.unwrap_or(0) != 0,
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
    #[arg(long)]
    pub series: Option<String>,
    #[arg(long)]
    pub box_set: Option<String>,
    #[arg(long = "genre")]
    pub genres: Vec<String>,
    #[arg(long = "studio")]
    pub studios: Vec<String>,
    #[arg(long = "format")]
    pub formats: Vec<String>,
    #[arg(long = "region")]
    pub regions: Vec<String>,
    #[arg(long = "screen-ratio")]
    pub screen_ratios: Vec<String>,
    #[arg(long = "subtitle")]
    pub subtitles: Vec<String>,
    #[arg(long = "audio-track")]
    pub audio_tracks: Vec<String>,
    #[arg(long = "hdr")]
    pub hdr: Vec<String>,
    #[arg(long = "extra")]
    pub extras: Vec<String>,
    #[arg(long = "tag")]
    pub tags: Vec<String>,
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

/// Whether `name` already exists for a two-hop lookup kind — read-only,
/// never creates anything (unlike `resolve_or_create_lookup`).
fn lookup_exists(conn: &Connection, table: &str, name: &str) -> Result<bool> {
    let exists: Option<i64> = conn
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
    Ok(exists.is_some())
}

/// Same as `lookup_exists`, but for person-like entities (`Actor`,
/// `CreditPerson`), via `ZPERSONDELEGATE`.
fn person_exists(conn: &Connection, table: &str, name: &str) -> Result<bool> {
    let exists: Option<i64> = conn
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
    Ok(exists.is_some())
}

/// One field as it would be written, and whether resolving it would
/// create a new lookup/person value.
#[derive(Serialize, Clone)]
pub struct ResolvedField {
    pub label: String,
    pub value: String,
    pub is_new: bool,
}

#[derive(Serialize)]
pub struct AddMoviePreview {
    pub title: String,
    pub fields: Vec<ResolvedField>,
    pub new_count: usize,
}

/// Resolves a new movie against the collection **without writing
/// anything** — same exact-match resolution `build_plan` itself uses,
/// just reporting what would happen instead of staging it. Used by
/// the GUI to show every resolved value with a "new" badge before the
/// user confirms the actual write.
pub fn preview_add_movie(conn: &Connection, fields: &MovieFields) -> Result<AddMoviePreview> {
    let title = fields.title.clone().context("title is required to add a movie")?;
    let mut resolved = Vec::new();

    if let Some(name) = &fields.country {
        resolved.push(ResolvedField {
            label: "Country".to_string(),
            value: name.clone(),
            is_new: !lookup_exists(conn, "ZCOUNTRY", name)?,
        });
    }
    if let Some(name) = &fields.series {
        resolved.push(ResolvedField {
            label: "Series".to_string(),
            value: name.clone(),
            is_new: !lookup_exists(conn, "ZSERIES", name)?,
        });
    }
    if let Some(name) = &fields.box_set {
        resolved.push(ResolvedField {
            label: "Box set".to_string(),
            value: name.clone(),
            is_new: !lookup_exists(conn, "ZBOXSET", name)?,
        });
    }
    for (label, table, names) in [
        ("Genre", "ZGENRE", &fields.genres),
        ("Studio", "ZSTUDIO", &fields.studios),
        ("Format", "ZFORMAT", &fields.formats),
        ("Region", "ZREGION", &fields.regions),
        ("Screen ratio", "ZSCREENRATIO", &fields.screen_ratios),
        ("Subtitle", "ZSUBTITLE", &fields.subtitles),
        ("Audio track", "ZAUDIOTRACK", &fields.audio_tracks),
        ("HDR", "ZHDR", &fields.hdr),
        ("Extra", "ZEXTRA", &fields.extras),
        ("Tag", "ZTAG", &fields.tags),
    ] {
        for name in names {
            resolved.push(ResolvedField {
                label: label.to_string(),
                value: name.clone(),
                is_new: !lookup_exists(conn, table, name)?,
            });
        }
    }
    for name in &fields.actors {
        resolved.push(ResolvedField {
            label: "Actor".to_string(),
            value: name.clone(),
            is_new: !person_exists(conn, "ZACTOR", name)?,
        });
    }
    for entry in &fields.crew {
        let name = entry.split_once(':').map(|(n, _)| n).unwrap_or(entry);
        resolved.push(ResolvedField {
            label: "Crew".to_string(),
            value: entry.clone(),
            is_new: !person_exists(conn, "ZCREDITPERSON", name)?,
        });
    }

    let new_count = resolved.iter().filter(|f| f.is_new).count();
    Ok(AddMoviePreview {
        title,
        fields: resolved,
        new_count,
    })
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
    let series_pk = fields
        .series
        .as_deref()
        .map(|name| resolve_or_create_lookup(conn, &mut alloc, "ZSERIES", "Series", name, &mut create_sql))
        .transpose()?;
    let box_set_pk = fields
        .box_set
        .as_deref()
        .map(|name| resolve_or_create_lookup(conn, &mut alloc, "ZBOXSET", "Boxset", name, &mut create_sql))
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
    let region_pks = fields
        .regions
        .iter()
        .map(|name| resolve_or_create_lookup(conn, &mut alloc, "ZREGION", "Region", name, &mut create_sql))
        .collect::<Result<Vec<_>>>()?;
    let screen_ratio_pks = fields
        .screen_ratios
        .iter()
        .map(|name| resolve_or_create_lookup(conn, &mut alloc, "ZSCREENRATIO", "ScreenRatio", name, &mut create_sql))
        .collect::<Result<Vec<_>>>()?;
    let subtitle_pks = fields
        .subtitles
        .iter()
        .map(|name| resolve_or_create_lookup(conn, &mut alloc, "ZSUBTITLE", "Subtitle", name, &mut create_sql))
        .collect::<Result<Vec<_>>>()?;
    let audio_track_pks = fields
        .audio_tracks
        .iter()
        .map(|name| resolve_or_create_lookup(conn, &mut alloc, "ZAUDIOTRACK", "AudioTrack", name, &mut create_sql))
        .collect::<Result<Vec<_>>>()?;
    let hdr_pks = fields
        .hdr
        .iter()
        .map(|name| resolve_or_create_lookup(conn, &mut alloc, "ZHDR", "HDR", name, &mut create_sql))
        .collect::<Result<Vec<_>>>()?;
    let extra_pks = fields
        .extras
        .iter()
        .map(|name| resolve_or_create_lookup(conn, &mut alloc, "ZEXTRA", "Extra", name, &mut create_sql))
        .collect::<Result<Vec<_>>>()?;
    let tag_pks = fields
        .tags
        .iter()
        .map(|name| resolve_or_create_lookup(conn, &mut alloc, "ZTAG", "Tag", name, &mut create_sql))
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
         ZCOLLECTION, ZSERIES, ZBOXSET, ZRELEASEYEAR, ZMYRATING, ZNOTES, ZPLOT, ZUPC, ZDATEADDED) \
         VALUES ({movie_pk}, {ent_movie}, 1, '{title}', {title_original}, {country}, \
         {collection}, {series}, {box_set}, {release_year}, {rating}, {notes}, {plot}, {upc}, {date_added});",
        ent_movie = ENT_MOVIE,
        title = sql_escape(&title),
        title_original = sql_literal(&fields.title_original),
        country = country_pk.map(|p| p.to_string()).unwrap_or_else(|| "NULL".to_string()),
        collection = collection_pk.map(|p| p.to_string()).unwrap_or_else(|| "NULL".to_string()),
        series = series_pk.map(|p| p.to_string()).unwrap_or_else(|| "NULL".to_string()),
        box_set = box_set_pk.map(|p| p.to_string()).unwrap_or_else(|| "NULL".to_string()),
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
    for pk in &region_pks {
        statements.push(format!(
            "INSERT INTO Z_7REGIONS (Z_35REGIONS, Z_7MOVIES8) VALUES ({pk}, {movie_pk});"
        ));
    }
    for pk in &screen_ratio_pks {
        statements.push(format!(
            "INSERT INTO Z_7SCREENRATIOS (Z_36SCREENRATIOS, Z_7MOVIES6) VALUES ({pk}, {movie_pk});"
        ));
    }
    for pk in &subtitle_pks {
        statements.push(format!(
            "INSERT INTO Z_7SUBTITLES (Z_42SUBTITLES, Z_7MOVIES5) VALUES ({pk}, {movie_pk});"
        ));
    }
    for pk in &audio_track_pks {
        statements.push(format!(
            "INSERT INTO Z_3MOVIES (Z_3AUDIOTRACKS, Z_7MOVIES) VALUES ({pk}, {movie_pk});"
        ));
    }
    for pk in &hdr_pks {
        statements.push(format!(
            "INSERT INTO Z_21MOVIES (Z_21HDRS, Z_7MOVIES4) VALUES ({pk}, {movie_pk});"
        ));
    }
    for pk in &extra_pks {
        statements.push(format!(
            "INSERT INTO Z_17MOVIES (Z_17EXTRAS, Z_7MOVIES1) VALUES ({pk}, {movie_pk});"
        ));
    }
    for pk in &tag_pks {
        statements.push(format!(
            "INSERT INTO Z_6TAGS (Z_43TAGS, Z_6COLLECTIBLES1) VALUES ({pk}, {movie_pk});"
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
    pub replace_regions: bool,
    #[arg(long)]
    pub replace_screen_ratios: bool,
    #[arg(long)]
    pub replace_subtitles: bool,
    #[arg(long)]
    pub replace_audio_tracks: bool,
    #[arg(long)]
    pub replace_hdr: bool,
    #[arg(long)]
    pub replace_extras: bool,
    #[arg(long)]
    pub replace_tags: bool,
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
    if let Some(name) = &fields.series {
        let pk = resolve_or_create_lookup(conn, &mut alloc, "ZSERIES", "Series", name, &mut statements)?;
        set_clauses.push(format!("ZSERIES = {pk}"));
    }
    if let Some(name) = &fields.box_set {
        let pk = resolve_or_create_lookup(conn, &mut alloc, "ZBOXSET", "Boxset", name, &mut statements)?;
        set_clauses.push(format!("ZBOXSET = {pk}"));
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
    if replace.replace_regions {
        statements.push(format!("DELETE FROM Z_7REGIONS WHERE Z_7MOVIES8 = {id};"));
        for name in &fields.regions {
            let pk = resolve_or_create_lookup(conn, &mut alloc, "ZREGION", "Region", name, &mut statements)?;
            statements.push(format!("INSERT INTO Z_7REGIONS (Z_35REGIONS, Z_7MOVIES8) VALUES ({pk}, {id});"));
        }
    }
    if replace.replace_screen_ratios {
        statements.push(format!("DELETE FROM Z_7SCREENRATIOS WHERE Z_7MOVIES6 = {id};"));
        for name in &fields.screen_ratios {
            let pk = resolve_or_create_lookup(conn, &mut alloc, "ZSCREENRATIO", "ScreenRatio", name, &mut statements)?;
            statements.push(format!(
                "INSERT INTO Z_7SCREENRATIOS (Z_36SCREENRATIOS, Z_7MOVIES6) VALUES ({pk}, {id});"
            ));
        }
    }
    if replace.replace_subtitles {
        statements.push(format!("DELETE FROM Z_7SUBTITLES WHERE Z_7MOVIES5 = {id};"));
        for name in &fields.subtitles {
            let pk = resolve_or_create_lookup(conn, &mut alloc, "ZSUBTITLE", "Subtitle", name, &mut statements)?;
            statements.push(format!("INSERT INTO Z_7SUBTITLES (Z_42SUBTITLES, Z_7MOVIES5) VALUES ({pk}, {id});"));
        }
    }
    if replace.replace_audio_tracks {
        statements.push(format!("DELETE FROM Z_3MOVIES WHERE Z_7MOVIES = {id};"));
        for name in &fields.audio_tracks {
            let pk = resolve_or_create_lookup(conn, &mut alloc, "ZAUDIOTRACK", "AudioTrack", name, &mut statements)?;
            statements.push(format!("INSERT INTO Z_3MOVIES (Z_3AUDIOTRACKS, Z_7MOVIES) VALUES ({pk}, {id});"));
        }
    }
    if replace.replace_hdr {
        statements.push(format!("DELETE FROM Z_21MOVIES WHERE Z_7MOVIES4 = {id};"));
        for name in &fields.hdr {
            let pk = resolve_or_create_lookup(conn, &mut alloc, "ZHDR", "HDR", name, &mut statements)?;
            statements.push(format!("INSERT INTO Z_21MOVIES (Z_21HDRS, Z_7MOVIES4) VALUES ({pk}, {id});"));
        }
    }
    if replace.replace_extras {
        statements.push(format!("DELETE FROM Z_17MOVIES WHERE Z_7MOVIES1 = {id};"));
        for name in &fields.extras {
            let pk = resolve_or_create_lookup(conn, &mut alloc, "ZEXTRA", "Extra", name, &mut statements)?;
            statements.push(format!("INSERT INTO Z_17MOVIES (Z_17EXTRAS, Z_7MOVIES1) VALUES ({pk}, {id});"));
        }
    }
    if replace.replace_tags {
        statements.push(format!("DELETE FROM Z_6TAGS WHERE Z_6COLLECTIBLES1 = {id};"));
        for name in &fields.tags {
            let pk = resolve_or_create_lookup(conn, &mut alloc, "ZTAG", "Tag", name, &mut statements)?;
            statements.push(format!("INSERT INTO Z_6TAGS (Z_43TAGS, Z_6COLLECTIBLES1) VALUES ({pk}, {id});"));
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
/// crew, genre/studio/format/region/screen-ratio/subtitle/audio-track/
/// HDR/extra/tag links) — **not yet** the relations nothing here
/// models yet (discs, episodes, box sets, series, links), same
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
        format!("DELETE FROM Z_7REGIONS WHERE Z_7MOVIES8 = {id};"),
        format!("DELETE FROM Z_7SCREENRATIOS WHERE Z_7MOVIES6 = {id};"),
        format!("DELETE FROM Z_7SUBTITLES WHERE Z_7MOVIES5 = {id};"),
        format!("DELETE FROM Z_3MOVIES WHERE Z_7MOVIES = {id};"),
        format!("DELETE FROM Z_21MOVIES WHERE Z_7MOVIES4 = {id};"),
        format!("DELETE FROM Z_17MOVIES WHERE Z_7MOVIES1 = {id};"),
        format!("DELETE FROM Z_6TAGS WHERE Z_6COLLECTIBLES1 = {id};"),
        format!("DELETE FROM ZCOLLECTIBLE WHERE Z_PK = {id};"),
        "COMMIT;".to_string(),
    ]
    .join("\n")
}

/// Returns `~/Documents/Collectorz` if `path` looks like a real, live
/// Movie Collector collection — lets a caller react differently (the
/// CLI's `--force` override) instead of always refusing outright like
/// `ensure_not_real_collection` below. Same shape as BookNojo's own
/// `looks_like_real_collection`, just app-specific detection here.
pub fn looks_like_real_collection(path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let text = canonical.to_string_lossy().to_lowercase();
    if text.contains("collectorz") && (text.contains("movie collector") || text.contains("movie_collector")) {
        let home = std::env::var_os("HOME")?;
        Some(PathBuf::from(home).join("Documents").join("Collectorz"))
    } else {
        None
    }
}

/// Refuses to operate on what looks like a real, live Movie Collector
/// collection — only ever a test/working copy. Used by the app (which
/// never offers any override) and as the CLI's own default without
/// `--force`. Same hard rule as the BookNojo/MusicNojo projects' tools.
pub fn ensure_not_real_collection(path: &Path) -> Result<()> {
    if looks_like_real_collection(path).is_some() {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        anyhow::bail!(
            "{} looks like the real Movie Collector collection path \
             (~/Documents/Collectorz/...) — refused. Use a test copy only.",
            canonical.display()
        );
    }
    Ok(())
}

/// Checks whether Movie Collector itself is currently running, via
/// `pgrep -x`. Writing to `Movie_data` while the app holds it open
/// risks an inconsistent WAL, or the app silently overwriting our
/// changes on its next save — this must return `false` before any
/// real write proceeds. Same check and reasoning as BookNojo's own
/// `is_book_collector_running` (`pgrep -x "Movie Collector"` matches
/// the app bundle's exact `CFBundleExecutable` name, confirmed
/// directly from the installed app's own `Info.plist`).
pub fn is_movie_collector_running() -> Result<bool> {
    let output = std::process::Command::new("pgrep")
        .args(["-x", "Movie Collector"])
        .output()
        .context("running pgrep to check whether Movie Collector is open")?;
    // pgrep exits 0 when it found at least one matching process, 1 when
    // it found none — both are successful checks, not failures of pgrep
    // itself, so only its exit code (not a spawn error) distinguishes them.
    Ok(output.status.success())
}

/// The package's `Images/` folder, given either the package directory
/// or its `Movie_data` file directly.
pub fn images_dir(db: &Path) -> std::path::PathBuf {
    if db.is_dir() {
        db.join("Images")
    } else {
        db.parent()
            .map(|p| p.join("Images"))
            .unwrap_or_else(|| std::path::PathBuf::from("Images"))
    }
}

/// Lowercase alphanumeric words joined by underscores, truncated —
/// same shape as the filenames already used for existing covers
/// (observed in the package's own `Images/` folder), and the same
/// helper BookNojo/MusicNojo already use for theirs.
pub fn slugify(title: &str, max_len: usize) -> String {
    let mut slug = String::new();
    for c in title.chars() {
        if c.is_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('_') && !slug.is_empty() {
            slug.push('_');
        }
    }
    let slug = slug.trim_matches('_');
    slug.chars().take(max_len).collect()
}

/// Copies `front`/`back` into the collection's `Images/` folder and
/// points `ZCOLLECTIBLE.ZFRONTCOVER`/`ZBACKCOVER` at the new
/// filenames — same shape as MusicNojo's own `set_cover` (the column
/// stores a bare filename directly, no extra lookup hop, same as
/// every other scalar field on `ZCOLLECTIBLE`). Does its own safety
/// checks rather than relying on the caller, since this isn't a
/// plan-then-commit write like `add`/`edit`/`delete` — there's no SQL
/// text to preview, just a file copy plus one `UPDATE`. `force` skips
/// the real-collection refusal — only `vc_add`'s own CLI ever passes
/// `true`, and only after its own typed-confirmation gate; the app
/// always passes `false`, no override available there.
pub fn set_cover(
    db: &Path,
    movie_id: i64,
    front: Option<&Path>,
    back: Option<&Path>,
    force: bool,
) -> Result<()> {
    if !force {
        ensure_not_real_collection(db)?;
    }
    if is_movie_collector_running()? {
        anyhow::bail!("Movie Collector is currently running — close it before writing.");
    }
    let db_path = resolve_db_path(db);
    let images = images_dir(db);
    std::fs::create_dir_all(&images)?;
    backup_database(db)?;

    let conn = Connection::open(&db_path)
        .with_context(|| format!("opening {}", db_path.display()))?;
    let title: String = conn
        .query_row(
            "SELECT ZTITLE FROM ZCOLLECTIBLE WHERE Z_PK = ?1 AND Z_ENT = ?2",
            rusqlite::params![movie_id, ENT_MOVIE],
            |row| row.get(0),
        )
        .with_context(|| format!("no movie with id {movie_id}"))?;
    let slug = slugify(&title, 28);

    for (source, side, column) in [(front, "f", "ZFRONTCOVER"), (back, "b", "ZBACKCOVER")] {
        if let Some(source) = source {
            let ext = source
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("jpg")
                .to_lowercase();
            let filename = format!("{movie_id}_{side}_{slug}.{ext}");
            std::fs::copy(source, images.join(&filename)).with_context(|| {
                format!("copying {} to Images/{filename}", source.display())
            })?;
            conn.execute(
                &format!("UPDATE ZCOLLECTIBLE SET {column} = ?1 WHERE Z_PK = ?2"),
                rusqlite::params![filename, movie_id],
            )?;
            println!("Images/{filename} <- {}", source.display());
        }
    }
    Ok(())
}

// A JSON-serializable shape close to `Movie`, but with `front_cover`
// resolved to an absolute `file://` URL (or omitted) instead of a bare
// filename — the live Tauri app resolves that itself via its asset
// protocol, but a standalone HTML file has no such mechanism.
#[derive(Serialize)]
struct HtmlMovie<'a> {
    id: i64,
    title: &'a str,
    title_original: &'a Option<String>,
    cast: &'a [CastMember],
    crew: &'a [Credit],
    discs: &'a [Disc],
    links: &'a [Link],
    genres: &'a [String],
    studios: &'a [String],
    formats: &'a [String],
    regions: &'a [String],
    screen_ratios: &'a [String],
    subtitles: &'a [String],
    audio_tracks: &'a [String],
    hdr: &'a [String],
    extras: &'a [String],
    tags: &'a [String],
    country: &'a Option<String>,
    collection: &'a Option<String>,
    series: &'a Option<String>,
    box_set: &'a Option<String>,
    is_tv_series: bool,
    date_added: &'a Option<String>,
    release_year: &'a Option<String>,
    rating: Option<i64>,
    notes: &'a Option<String>,
    plot: &'a Option<String>,
    front_cover_url: Option<String>,
}

// A `file://` URL needs an absolute path and percent-encoding of
// anything outside the unreserved set — same helper/lesson as
// MusicNojo's own static export (found via a screenshot of its very
// first generated page: a relative `--db` path silently produced a
// broken image, `file://sample-collection/...` parsing as host
// "sample-collection" with an empty path).
fn file_url(path: &str) -> String {
    const UNRESERVED: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~";
    let mut out = String::from("file://");
    for byte in path.bytes() {
        if byte == b'/' || UNRESERVED.contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Writes a standalone, read-only HTML page replicating the live
/// app's current feature set: search, sub-collection tabs, an A-Z
/// bar, a group-by sidebar (genre/studio/format/country), sort
/// presets plus clickable column headers, a list/grid toggle with
/// pagination, a detail card, and a statistics panel. No add/edit/
/// delete — `vc_add` already covers writing, directly and more
/// safely (a reviewable plan before any commit).
pub fn write_html(path: &Path, movies: &[Movie], images_dir: &Path) -> Result<()> {
    let images_dir_abs = images_dir.canonicalize().unwrap_or_else(|_| images_dir.to_path_buf());
    let images_dir_str = images_dir_abs.to_string_lossy();
    let html_movies: Vec<HtmlMovie> = movies
        .iter()
        .map(|m| HtmlMovie {
            id: m.id,
            title: &m.title,
            title_original: &m.title_original,
            cast: &m.cast,
            crew: &m.crew,
            discs: &m.discs,
            links: &m.links,
            genres: &m.genres,
            studios: &m.studios,
            formats: &m.formats,
            regions: &m.regions,
            screen_ratios: &m.screen_ratios,
            subtitles: &m.subtitles,
            audio_tracks: &m.audio_tracks,
            hdr: &m.hdr,
            extras: &m.extras,
            tags: &m.tags,
            country: &m.country,
            collection: &m.collection,
            series: &m.series,
            box_set: &m.box_set,
            is_tv_series: m.is_tv_series,
            date_added: &m.date_added,
            release_year: &m.release_year,
            rating: m.rating,
            notes: &m.notes,
            plot: &m.plot,
            front_cover_url: m
                .front_cover
                .as_deref()
                .map(|f| file_url(&format!("{images_dir_str}/{f}"))),
        })
        .collect();
    // Escaped so a title/plot containing "</script>" can't break out
    // of the embedding <script> tag.
    let data_json = serde_json::to_string(&html_movies)?.replace("</", "<\\/");

    let html = format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>VideoNojo — Collection</title>
<style>
{CSS}
</style>
</head>
<body>
<header>
  <h1>VideoNojo</h1>
  <input id="search" type="search" data-i18n-placeholder="search_placeholder" placeholder="Search by title or cast…" autofocus>
  <span id="count"></span>
  <button id="stats-btn" data-i18n="nav_stats">📊 Statistics</button>
  <span id="lang-toggle">
    <button id="lang-en" type="button">EN</button>
    <button id="lang-fr" type="button">FR</button>
  </span>
</header>
<div id="tabs"></div>
<div id="azbar"></div>
<main id="main">
  <aside id="groups">
    <select id="group-field">
      <option value="genres" data-i18n="opt_group_genres">Genre</option>
      <option value="studios" data-i18n="opt_group_studios">Studio</option>
      <option value="formats" data-i18n="opt_group_formats">Format</option>
      <option value="country" data-i18n="opt_group_country">Country</option>
    </select>
    <div id="group-list"></div>
  </aside>
  <div id="right-col">
    <div id="list-pane">
      <div id="list-toolbar">
        <select id="sort">
          <option value="title" data-i18n="opt_sort_title">Title (A→Z)</option>
          <option value="year_desc" data-i18n="opt_sort_year_desc">Year (newest first)</option>
          <option value="rating_desc" data-i18n="opt_sort_rating_desc">Rating (highest first)</option>
          <option value="country" data-i18n="opt_sort_country">Country (A→Z)</option>
          <option value="date_added_desc" data-i18n="opt_sort_date_added">Date added (newest first)</option>
        </select>
        <div id="view-toggle">
          <button id="view-list" class="active" data-i18n="list_view_list">☰ List</button>
          <button id="view-grid" data-i18n="list_view_grid">▦ Grid</button>
        </div>
      </div>
      <table id="movies">
        <thead><tr>
          <th data-k="title" data-i18n="col_title">Title</th>
          <th data-k="release_year" data-i18n="col_year">Year</th>
          <th data-k="country" data-i18n="col_country">Country</th>
          <th data-k="genres" data-i18n="col_genres">Genre(s)</th>
        </tr></thead>
        <tbody></tbody>
      </table>
      <div id="grid-view"></div>
      <div id="pager">
        <button id="prev-page" data-i18n="list_prev">‹ Prev.</button>
        <span id="page-info"></span>
        <button id="next-page" data-i18n="list_next">Next ›</button>
      </div>
    </div>
    <aside id="detail">
      <p id="detail-empty" data-i18n="detail_select">Select a movie.</p>
      <div id="detail-content"></div>
    </aside>
  </div>
</main>
<div id="stats-pane"></div>
<script id="data" type="application/json">{data_json}</script>
<script>
{JS}
</script>
</body>
</html>
"##,
        CSS = STATIC_EXPORT_CSS,
        JS = STATIC_EXPORT_JS,
        data_json = data_json,
    );
    std::fs::write(path, html).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

const STATIC_EXPORT_CSS: &str = r##"
body { font-family: -apple-system, system-ui, sans-serif; margin: 0; color: #1a1a1a; height: 100vh; display: flex; flex-direction: column; }
header { display: flex; align-items: center; gap: 0.6rem; padding: 0.75rem 1rem; border-bottom: 1px solid #ddd; background: #fff; flex-shrink: 0; }
header h1 { font-size: 1.1rem; margin: 0; flex-shrink: 0; }
#search { flex: 1 1 12rem; min-width: 8rem; max-width: 24rem; font-size: 1rem; padding: 0.4rem 0.6rem; }
#count { color: #666; font-size: 0.85rem; }
#stats-btn { padding: 0.4rem 0.8rem; cursor: pointer; margin-left: auto; flex-shrink: 0; white-space: nowrap; }
#stats-btn.active { background: #6a7fe0; color: #fff; border-color: #6a7fe0; }
#lang-toggle { display: flex; gap: 0.2rem; flex-shrink: 0; }
#lang-toggle button { padding: 0.25rem 0.5rem; font-size: 0.75rem; cursor: pointer; border: 1px solid #ccc; background: #fff; border-radius: 4px; }
#lang-toggle button.active { background: #2e4bb0; color: #fff; border-color: #2e4bb0; }
#tabs { display: flex; gap: 0.4rem; padding: 0.4rem 1rem; overflow-x: auto; background: #f0f2ff; border-bottom: 1px solid #ddd; flex-shrink: 0; }
#tabs .tab { flex: 0 0 auto; padding: 0.3rem 0.7rem; border-radius: 999px; font-size: 0.75rem; font-weight: 500; white-space: nowrap; cursor: pointer; background: #e4e9ff; border: 1px solid #c3d2f7; color: #2e4bb0; }
#tabs .tab:hover { background: #dce5fb; border-color: #6a7fe0; }
#tabs .tab.active { background: #6a7fe0; border-color: #6a7fe0; color: #fff; }
#tabs .tab .n { opacity: 0.75; margin-left: 0.3rem; }
#tabs .tab.active .n { opacity: 0.9; }
#azbar { display: flex; flex-wrap: nowrap; justify-content: space-between; overflow-x: auto; gap: 0.2rem; padding: 0.4rem 1rem; background: #fafafa; border-bottom: 1px solid #ddd; flex-shrink: 0; }
#azbar button { border: 1px solid transparent; background: none; font-size: 0.75rem; font-weight: 600; color: #888; padding: 0.25rem 0.55rem; border-radius: 4px; cursor: pointer; min-width: 1.6rem; flex-shrink: 0; }
#azbar button:hover { background: #f5f7ff; color: #1a1a1a; }
#azbar button.active { background: #6a7fe0; color: #fff; }
#main { flex: 1; min-height: 0; display: flex; overflow: hidden; }
#groups { flex: 0 0 220px; min-width: 140px; padding: 0.75rem; overflow-y: auto; background: #fafafa; }
#group-field { width: 100%; margin-bottom: 0.5rem; padding: 0.3rem; }
.group-row { display: flex; justify-content: space-between; gap: 0.5rem; padding: 0.3rem 0.4rem; border-radius: 4px; cursor: pointer; font-size: 0.9rem; }
.group-row:hover { background: #eef0ff; }
.group-row.active { background: #dce5fb; font-weight: 600; }
.group-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.group-count { color: #888; flex-shrink: 0; }
.group-count.sortable { cursor: pointer; padding: 0.05rem 0.35rem; border-radius: 999px; }
.group-count.sortable:hover { background: #e4e9ff; color: #2e4bb0; }
.group-count.sort-active { background: #6a7fe0; color: #fff; }
#right-col { flex: 1; min-width: 0; display: flex; flex-direction: column; overflow: hidden; }
#list-pane { flex: 0 0 240px; min-height: 100px; display: flex; flex-direction: column; overflow: auto; }
#list-toolbar { display: flex; justify-content: space-between; align-items: center; padding: 0.4rem 0.8rem; border-bottom: 1px solid #eee; background: #fafafa; flex-shrink: 0; }
#sort { padding: 0.25rem 0.4rem; font-size: 0.85rem; }
#view-toggle { display: flex; border: 1px solid #ddd; border-radius: 6px; overflow: hidden; }
#view-toggle button { border: none; background: #fff; padding: 0.25rem 0.5rem; font-size: 0.8rem; cursor: pointer; color: #666; }
#view-toggle button:hover { background: #f5f7ff; color: #1a1a1a; }
#view-toggle button + button { border-left: 1px solid #ddd; }
#view-toggle button.active { background: #6a7fe0; color: #fff; }
#movies { border-collapse: collapse; width: 100%; }
#movies th { text-align: left; position: sticky; top: 0; background: #f5f5f5; padding: 0.4rem 0.6rem; border-bottom: 1px solid #ddd; cursor: pointer; user-select: none; }
#movies td { padding: 0.4rem 0.6rem; border-bottom: 1px solid #eee; cursor: pointer; }
#movies tr:hover td { background: #f5f7ff; }
#grid-view { display: none; grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr)); gap: 0.8rem; padding: 0.8rem; align-content: start; }
#list-pane.view-grid table { display: none; }
#list-pane.view-grid #grid-view { display: grid; }
.grid-tile { cursor: pointer; border-radius: 6px; padding: 0.3rem; }
.grid-tile:hover { background: #f0f2ff; }
.grid-tile .thumb { width: 100%; aspect-ratio: 2 / 3; object-fit: cover; border-radius: 4px; background: #e4e4e8; display: block; }
.grid-tile .t { font-size: 0.8rem; font-weight: 600; margin-top: 0.3rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.grid-tile .y { font-size: 0.72rem; color: #666; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
#pager { display: flex; align-items: center; justify-content: center; gap: 0.8rem; padding: 0.4rem; border-top: 1px solid #eee; background: #fafafa; flex-shrink: 0; font-size: 0.8rem; }
#pager button { padding: 0.25rem 0.6rem; cursor: pointer; }
#pager button:disabled { cursor: default; opacity: 0.4; }
#page-info { color: #666; }
#detail { flex: 1 1 auto; min-height: 100px; padding: 1rem; overflow-y: auto; background: #181818; }
#detail-empty { color: #888; font-style: italic; }
#detail-content { background: #181818; color: #e6e6e6; padding: 1rem; border-radius: 6px; }
#detail-content h2 { font-size: 1.3rem; font-weight: 700; color: #4ea1f3; margin: 0 0 0.75rem; }
#detail-content img { max-width: 160px; display: block; margin-bottom: 0.75rem; border-radius: 4px; }
#detail-content dl { display: grid; grid-template-columns: 6rem 1fr; gap: 0.25rem 0.5rem; font-size: 0.9rem; }
#detail-content dt { color: #999; }
#detail-content dd { margin: 0; }
#detail-content .disc-heading { font-weight: 600; margin-top: 0.75rem; font-size: 0.85rem; color: #999; }
#detail-content ol { padding-left: 1.2rem; font-size: 0.9rem; margin: 0.25rem 0 0; }
.link-list { list-style: none; margin: 0; padding: 0; }
.link-list a { color: #4ea1f3; }
.link-list a:hover { text-decoration: underline; }
#stats-pane { display: none; flex: 1; overflow-y: auto; background: #fbfbfc; padding: 1.5rem; min-height: 0; }
body.panel-stats #main { display: none; }
body.panel-stats #stats-pane { display: block; }
.stats-wrap { max-width: 60rem; margin: 0 auto; }
.stats-wrap h2 { font-size: 1.1rem; margin: 0 0 0.9rem; }
.stat-summary { display: flex; flex-wrap: wrap; gap: 1rem; font-size: 0.85rem; color: #666; margin-bottom: 1.2rem; }
.stat-summary strong { color: #1a1a1a; }
.stat-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(20rem, 1fr)); gap: 1.1rem; }
.stat-block { background: #fff; border: 1px solid #ddd; border-radius: 8px; padding: 0.9rem 1rem; }
.stat-block h3 { font-size: 0.7rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; color: #888; margin: 0 0 0.6rem; }
.stat-row { display: flex; align-items: center; gap: 0.5rem; font-size: 0.8rem; margin-bottom: 0.3rem; }
.stat-row .label { width: 9rem; flex-shrink: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.stat-row .bar-wrap { flex: 1; background: #eceef2; border-radius: 3px; height: 0.6rem; overflow: hidden; }
.stat-row .bar { display: block; background: #6a7fe0; height: 100%; }
.stat-row .val { width: 2.3rem; text-align: right; color: #888; font-size: 0.7rem; flex-shrink: 0; }
.stat-empty { font-size: 0.8rem; color: #888; }
"##;

const STATIC_EXPORT_JS: &str = r##"
const movies = JSON.parse(document.getElementById("data").textContent);
let currentDetailId = null;
let groupField = "genres";
let selectedGroup = null;
let selectedLetter = null;
let groupSortByCount = false;
let sortKey = "title", sortAsc = true;
let viewMode = "list", page = 1, panel = "library";
const PAGE_SIZE = 60;
const DEFAULT_COLLECTION_KEY = "__default__";
let currentCollection = null;

// Same i18n shape and FR copy as the app itself (app/src/app.js) — kept
// deliberately separate from that larger dictionary since this export
// only ever needs the read/browse-only subset of keys (no add/edit/
// delete, no cover-setting).
const I18N = {
  en: {
    search_placeholder: "Search by title or cast…",
    count_movies: "{n} movie(s)",
    nav_stats: "📊 Statistics",
    nav_back_to_collection: "← Back to collection",
    group_all: "All",
    azbar_all: "All",
    group_sort_tooltip_active: "Sorted by count — click to sort alphabetically",
    group_sort_tooltip_inactive: "Sorted alphabetically — click to sort by count",
    collection_default: "My Collection",
    opt_group_genres: "Genre",
    opt_group_studios: "Studio",
    opt_group_formats: "Format",
    opt_group_country: "Country",
    opt_sort_title: "Title (A→Z)",
    opt_sort_year_desc: "Year (newest first)",
    opt_sort_rating_desc: "Rating (highest first)",
    opt_sort_country: "Country (A→Z)",
    opt_sort_date_added: "Date added (newest first)",
    list_view_list: "☰ List",
    list_view_grid: "▦ Grid",
    list_prev: "‹ Prev.",
    list_next: "Next ›",
    list_page_info: "Page {page} of {total}",
    col_title: "Title",
    col_year: "Year",
    col_country: "Country",
    col_genres: "Genre(s)",
    detail_select: "Select a movie.",
    yes: "Yes",
    detail_field_original_title: "Original title",
    detail_field_year: "Year",
    detail_field_country: "Country",
    detail_field_series: "Series",
    detail_field_box_set: "Box set",
    detail_field_tv_series: "TV series",
    detail_field_genres: "Genre(s)",
    detail_field_studios: "Studio(s)",
    detail_field_formats: "Format(s)",
    detail_field_regions: "Region(s)",
    detail_field_screen_ratios: "Screen ratio(s)",
    detail_field_subtitles: "Subtitle(s)",
    detail_field_audio_tracks: "Audio track(s)",
    detail_field_hdr: "HDR",
    detail_field_extras: "Extra(s)",
    detail_field_tags: "Tag(s)",
    detail_field_cast: "Cast",
    detail_field_crew: "Crew",
    detail_field_rating: "Rating",
    detail_field_notes: "Notes",
    detail_field_plot: "Plot",
    detail_field_date_added: "Date added",
    detail_field_links: "Links",
    stats_title: "Statistics — {name}",
    stats_movies: "{n} movie(s)",
    stats_no_cover: "{n} with no cover",
    stats_no_cast: "{n} with no cast",
    stats_genres: "Genres",
    stats_studios: "Studios",
    stats_formats: "Formats",
    stats_countries: "Countries",
    stats_by_decade: "By release decade",
    stats_by_added_year: "By year added",
    stats_no_data: "No data.",
  },
  fr: {
    search_placeholder: "Rechercher par titre ou distribution…",
    count_movies: "{n} film(s)",
    nav_stats: "📊 Statistiques",
    nav_back_to_collection: "← Retour à la collection",
    group_all: "Tous",
    azbar_all: "Tout",
    group_sort_tooltip_active: "Trié par nombre — cliquer pour trier par ordre alphabétique",
    group_sort_tooltip_inactive: "Trié par ordre alphabétique — cliquer pour trier par nombre",
    collection_default: "Ma collection",
    opt_group_genres: "Genre",
    opt_group_studios: "Studio",
    opt_group_formats: "Format",
    opt_group_country: "Pays",
    opt_sort_title: "Titre (A→Z)",
    opt_sort_year_desc: "Année (plus récent d'abord)",
    opt_sort_rating_desc: "Note (plus élevée d'abord)",
    opt_sort_country: "Pays (A→Z)",
    opt_sort_date_added: "Date d'ajout (plus récent d'abord)",
    list_view_list: "☰ Liste",
    list_view_grid: "▦ Grille",
    list_prev: "‹ Préc.",
    list_next: "Suiv. ›",
    list_page_info: "Page {page} / {total}",
    col_title: "Titre",
    col_year: "Année",
    col_country: "Pays",
    col_genres: "Genre(s)",
    detail_select: "Sélectionnez un film.",
    yes: "Oui",
    detail_field_original_title: "Titre original",
    detail_field_year: "Année",
    detail_field_country: "Pays",
    detail_field_series: "Série",
    detail_field_box_set: "Coffret",
    detail_field_tv_series: "Série TV",
    detail_field_genres: "Genre(s)",
    detail_field_studios: "Studio(s)",
    detail_field_formats: "Format(s)",
    detail_field_regions: "Région(s)",
    detail_field_screen_ratios: "Format(s) d'écran",
    detail_field_subtitles: "Sous-titre(s)",
    detail_field_audio_tracks: "Piste(s) audio",
    detail_field_hdr: "HDR",
    detail_field_extras: "Bonus",
    detail_field_tags: "Étiquette(s)",
    detail_field_cast: "Distribution",
    detail_field_crew: "Équipe technique",
    detail_field_rating: "Note",
    detail_field_notes: "Notes",
    detail_field_plot: "Synopsis",
    detail_field_date_added: "Date d'ajout",
    detail_field_links: "Liens",
    stats_title: "Statistiques — {name}",
    stats_movies: "{n} film(s)",
    stats_no_cover: "{n} sans jaquette",
    stats_no_cast: "{n} sans distribution",
    stats_genres: "Genres",
    stats_studios: "Studios",
    stats_formats: "Formats",
    stats_countries: "Pays",
    stats_by_decade: "Par décennie de sortie",
    stats_by_added_year: "Par année d'ajout",
    stats_no_data: "Aucune donnée.",
  },
};
let lang = "en";
function t(key, vars) {
  let text = (I18N[lang] && I18N[lang][key]) || I18N.en[key] || key;
  if (vars) for (const [k, v] of Object.entries(vars)) text = text.replace(`{${k}}`, v);
  return text;
}
function applyStaticLabels() {
  document.querySelectorAll("[data-i18n]").forEach((el) => { el.textContent = t(el.dataset.i18n); });
  document.querySelectorAll("[data-i18n-placeholder]").forEach((el) => { el.placeholder = t(el.dataset.i18nPlaceholder); });
  document.getElementById("lang-en").classList.toggle("active", lang === "en");
  document.getElementById("lang-fr").classList.toggle("active", lang === "fr");
  document.documentElement.lang = lang;
}
function setLang(newLang) {
  lang = newLang;
  applyStaticLabels();
  renderTabs();
  renderGroups();
  render(document.getElementById("search").value);
  if (currentDetailId !== null) showDetail(currentDetailId);
  if (panel === "stats") renderStats();
}
document.getElementById("lang-en").addEventListener("click", () => setLang("en"));
document.getElementById("lang-fr").addEventListener("click", () => setLang("fr"));

function escapeHtml(s) {
  return String(s ?? "").replace(/[&<>"']/g, (c) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[c]);
}

function field(label, value) {
  return value ? `<dt>${label}</dt><dd>${escapeHtml(value)}</dd>` : "";
}

function roleList(label, names) {
  return names && names.length ? `<dt>${label}</dt><dd>${escapeHtml(names.join(", "))}</dd>` : "";
}

function creditList(label, credits) {
  if (!credits || !credits.length) return "";
  const text = credits.map((c) => (c.role ? `${c.name} (${c.role})` : c.name)).join(", ");
  return `<dt>${label}</dt><dd>${escapeHtml(text)}</dd>`;
}

function linksBlock(label, links) {
  const items = (links || []).filter((l) => l.url).map((l) =>
    `<li><a href="${escapeHtml(l.url)}" target="_blank" rel="noopener">${escapeHtml(l.description || l.url)}</a></li>`
  ).join("");
  return items ? `<dt>${label}</dt><dd><ul class="link-list">${items}</ul></dd>` : "";
}

function discsBlock(discs) {
  const interesting = (discs || []).filter(
    (d) => (d.episodes && d.episodes.length) || d.format || d.storage_device || d.extra_features || d.storage_slot
  );
  if (!interesting.length) return "";
  return interesting.map((d) => {
    const meta = [d.format, d.storage_device, d.extra_features, d.storage_slot].filter(Boolean).map(escapeHtml).join(" — ");
    const episodes = (d.episodes || []).map((e) => {
      const airDate = [e.first_air_year, e.first_air_month, e.first_air_day].filter(Boolean).join("-");
      return `<li>${escapeHtml(e.title || "")}${airDate ? ` — ${escapeHtml(airDate)}` : ""}</li>`;
    }).join("");
    return `<div class="disc-heading">${escapeHtml(d.title || "")}${meta ? ` (${meta})` : ""}</div>${episodes ? `<ol>${episodes}</ol>` : ""}`;
  }).join("");
}

function collectionKey(m) {
  return m.collection || DEFAULT_COLLECTION_KEY;
}
function collectionLabel(key) {
  return key === DEFAULT_COLLECTION_KEY ? t("collection_default") : key;
}

function groupValues(m, f) {
  return f === "country" ? (m.country ? [m.country] : []) : m[f] || [];
}

function computeCollections() {
  const counts = new Map();
  for (const m of movies) {
    const key = collectionKey(m);
    counts.set(key, (counts.get(key) || 0) + 1);
  }
  return [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
}

function renderTabs() {
  const collections = computeCollections();
  if (currentCollection === null || !collections.some(([key]) => key === currentCollection)) {
    currentCollection = collections.length ? collections[0][0] : null;
  }
  document.getElementById("tabs").innerHTML = collections
    .map(([key, count]) => `
      <div class="tab ${key === currentCollection ? "active" : ""}" data-key="${escapeHtml(key)}">
        ${escapeHtml(collectionLabel(key))} <span class="n">${count}</span>
      </div>`)
    .join("");
  document.querySelectorAll("#tabs .tab").forEach((el) =>
    el.addEventListener("click", () => {
      currentCollection = el.dataset.key;
      selectedGroup = null;
      selectedLetter = null;
      page = 1;
      renderTabs();
      renderGroups();
      render(document.getElementById("search").value);
      if (panel === "stats") renderStats();
    })
  );
}

function scopedMovies() {
  return movies.filter((m) => collectionKey(m) === currentCollection);
}

// First letter used by the A-Z quick-jump bar — non-letters (numbers,
// symbols, accented-but-not-matched edge cases) all bucket under "#".
function letterOf(value) {
  const ch = (value || "").trim().charAt(0).toUpperCase();
  return /[A-Z]/.test(ch) ? ch : "#";
}

function computeAvailableLetters() {
  const set = new Set();
  for (const m of scopedMovies()) {
    for (const v of groupValues(m, groupField)) if (v) set.add(letterOf(v));
  }
  return [...set].sort();
}

function renderAzBar() {
  const el = document.getElementById("azbar");
  el.innerHTML = "";
  const all = document.createElement("button");
  all.textContent = t("azbar_all");
  all.className = selectedLetter === null ? "active" : "";
  all.addEventListener("click", () => { selectedLetter = null; renderGroups(); });
  el.appendChild(all);
  for (const letter of computeAvailableLetters()) {
    const btn = document.createElement("button");
    btn.textContent = letter;
    btn.className = selectedLetter === letter ? "active" : "";
    btn.addEventListener("click", () => { selectedLetter = letter; renderGroups(); });
    el.appendChild(btn);
  }
}

function computeGroups() {
  const counts = new Map();
  for (const m of scopedMovies()) {
    for (const v of groupValues(m, groupField)) {
      if (selectedLetter && letterOf(v) !== selectedLetter) continue;
      counts.set(v, (counts.get(v) || 0) + 1);
    }
  }
  return groupSortByCount
    ? [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    : [...counts.entries()].sort((a, b) => a[0].localeCompare(b[0]));
}

function renderGroups() {
  renderAzBar();
  const groups = computeGroups();
  const allClass = selectedGroup === null ? "active" : "";
  const rows = groups
    .map(([value, count]) => `
      <div class="group-row ${selectedGroup === value ? "active" : ""}" data-value="${escapeHtml(value)}">
        <span class="group-label">${escapeHtml(value)}</span>
        <span class="group-count">${count}</span>
      </div>`)
    .join("");
  document.getElementById("group-list").innerHTML = `
    <div class="group-row ${allClass}" data-value="">
      <span class="group-label">${t("group_all")}</span>
      <span class="group-count sortable${groupSortByCount ? " sort-active" : ""}" id="group-sort-toggle" title="${groupSortByCount ? t("group_sort_tooltip_active") : t("group_sort_tooltip_inactive")}">${scopedMovies().length}</span>
    </div>
    ${rows}
  `;
  document.querySelectorAll("#group-list .group-row").forEach((el) =>
    el.addEventListener("click", () => {
      selectedGroup = el.dataset.value || null;
      page = 1;
      renderGroups();
      render(document.getElementById("search").value);
    })
  );
  document.getElementById("group-sort-toggle").addEventListener("click", (e) => {
    e.stopPropagation();
    groupSortByCount = !groupSortByCount;
    renderGroups();
  });
}

const SORT_PRESETS = {
  title: { key: "title", asc: true },
  year_desc: { key: "release_year", asc: false },
  rating_desc: { key: "rating", asc: false },
  country: { key: "country", asc: true },
  date_added_desc: { key: "date_added", asc: false },
};
const sortEl = document.getElementById("sort");
function syncSortDropdown() {
  const match = Object.entries(SORT_PRESETS).find(([, p]) => p.key === sortKey && p.asc === sortAsc);
  if (match) sortEl.value = match[0];
}
sortEl.addEventListener("change", () => {
  const preset = SORT_PRESETS[sortEl.value];
  sortKey = preset.key; sortAsc = preset.asc; page = 1;
  render(document.getElementById("search").value);
});
document.querySelectorAll("#movies th").forEach((th) => th.addEventListener("click", () => {
  const k = th.dataset.k;
  sortAsc = sortKey === k ? !sortAsc : true;
  sortKey = k; page = 1;
  syncSortDropdown();
  render(document.getElementById("search").value);
}));

function sortRows(rows) {
  rows.sort((a, b) => {
    const va = Array.isArray(a[sortKey]) ? a[sortKey].join(", ") : a[sortKey];
    const vb = Array.isArray(b[sortKey]) ? b[sortKey].join(", ") : b[sortKey];
    const cmp = typeof va === "number" || typeof vb === "number"
      ? (va ?? -Infinity) - (vb ?? -Infinity)
      : String(va ?? "").localeCompare(String(vb ?? ""));
    return sortAsc ? cmp : -cmp;
  });
  return rows;
}

function render(filterText) {
  const filter = (filterText || "").toLowerCase();
  const rows = sortRows(
    scopedMovies()
      .filter(
        (m) =>
          !filter ||
          m.title.toLowerCase().includes(filter) ||
          (m.cast || []).map((c) => c.name).join(" ").toLowerCase().includes(filter)
      )
      .filter((m) => selectedGroup === null || groupValues(m, groupField).includes(selectedGroup))
  );

  const totalPages = Math.max(1, Math.ceil(rows.length / PAGE_SIZE));
  page = Math.min(Math.max(1, page), totalPages);
  const start = (page - 1) * PAGE_SIZE;
  const pageRows = rows.slice(start, start + PAGE_SIZE);

  document.getElementById("count").textContent = t("count_movies", { n: rows.length });
  const tbody = document.querySelector("#movies tbody");
  tbody.innerHTML = pageRows
    .map((m) => `
      <tr data-id="${m.id}">
        <td>${escapeHtml(m.title)}</td>
        <td>${escapeHtml(m.release_year)}</td>
        <td>${escapeHtml(m.country)}</td>
        <td>${escapeHtml((m.genres || []).join(", "))}</td>
      </tr>`)
    .join("");
  tbody.querySelectorAll("tr").forEach((tr) =>
    tr.addEventListener("click", () => showDetail(Number(tr.dataset.id)))
  );

  const grid = document.getElementById("grid-view");
  grid.innerHTML = pageRows.map((m) => {
    const thumb = m.front_cover_url ? `<img class="thumb" loading="lazy" src="${m.front_cover_url}" alt="">` : `<div class="thumb"></div>`;
    return `<div class="grid-tile" data-id="${m.id}">${thumb}<div class="t">${escapeHtml(m.title)}</div><div class="y">${escapeHtml(m.release_year)}</div></div>`;
  }).join("");
  grid.querySelectorAll(".grid-tile").forEach((tile) =>
    tile.addEventListener("click", () => showDetail(Number(tile.dataset.id)))
  );

  document.getElementById("page-info").textContent = t("list_page_info", { page, total: totalPages });
  document.getElementById("prev-page").disabled = page <= 1;
  document.getElementById("next-page").disabled = page >= totalPages;
}

document.getElementById("prev-page").addEventListener("click", () => { page--; render(document.getElementById("search").value); });
document.getElementById("next-page").addEventListener("click", () => { page++; render(document.getElementById("search").value); });

const viewListBtn = document.getElementById("view-list");
const viewGridBtn = document.getElementById("view-grid");
const listPaneEl = document.getElementById("list-pane");
function setViewMode(mode) {
  viewMode = mode;
  listPaneEl.classList.toggle("view-grid", mode === "grid");
  viewListBtn.classList.toggle("active", mode === "list");
  viewGridBtn.classList.toggle("active", mode === "grid");
}
viewListBtn.addEventListener("click", () => setViewMode("list"));
viewGridBtn.addEventListener("click", () => setViewMode("grid"));

function showDetail(id) {
  currentDetailId = id;
  const m = movies.find((x) => x.id === id);
  if (!m) return;
  document.getElementById("detail-empty").hidden = true;
  document.getElementById("detail-content").innerHTML = `
    ${m.front_cover_url ? `<img src="${m.front_cover_url}" alt="">` : ""}
    <h2>${escapeHtml(m.title)}</h2>
    <dl>
      ${field(t("detail_field_original_title"), m.title_original)}
      ${field(t("detail_field_year"), m.release_year)}
      ${field(t("detail_field_country"), m.country)}
      ${field(t("detail_field_series"), m.series)}
      ${field(t("detail_field_box_set"), m.box_set)}
      ${field(t("detail_field_tv_series"), m.is_tv_series ? t("yes") : "")}
      ${roleList(t("detail_field_genres"), m.genres)}
      ${roleList(t("detail_field_studios"), m.studios)}
      ${roleList(t("detail_field_formats"), m.formats)}
      ${roleList(t("detail_field_regions"), m.regions)}
      ${roleList(t("detail_field_screen_ratios"), m.screen_ratios)}
      ${roleList(t("detail_field_subtitles"), m.subtitles)}
      ${roleList(t("detail_field_audio_tracks"), m.audio_tracks)}
      ${roleList(t("detail_field_hdr"), m.hdr)}
      ${roleList(t("detail_field_extras"), m.extras)}
      ${roleList(t("detail_field_tags"), m.tags)}
      ${roleList(t("detail_field_cast"), (m.cast || []).map((c) => c.name))}
      ${creditList(t("detail_field_crew"), m.crew)}
      ${field(t("detail_field_rating"), m.rating != null ? String(m.rating) : "")}
      ${field(t("detail_field_notes"), m.notes)}
      ${field(t("detail_field_plot"), m.plot)}
      ${field(t("detail_field_date_added"), m.date_added)}
      ${linksBlock(t("detail_field_links"), m.links)}
    </dl>
    ${discsBlock(m.discs)}
  `;
}

document.getElementById("search").addEventListener("input", (e) => { page = 1; render(e.target.value); });
document.getElementById("group-field").addEventListener("change", (e) => {
  groupField = e.target.value; selectedGroup = null; selectedLetter = null; page = 1;
  renderGroups(); render(document.getElementById("search").value);
});

// --- Statistics ---------------------------------------------------------
function computeStats() {
  const scoped = scopedMovies();
  const tally = (getter) => {
    const m = new Map();
    for (const movie of scoped) for (const v of getter(movie)) if (v) m.set(v, (m.get(v) || 0) + 1);
    return [...m.entries()].sort((a, b) => b[1] - a[1]);
  };
  const decades = new Map();
  for (const movie of scoped) {
    const y = parseInt(movie.release_year, 10);
    if (!y) continue;
    const d = Math.floor(y / 10) * 10;
    decades.set(d, (decades.get(d) || 0) + 1);
  }
  const addedYears = new Map();
  for (const movie of scoped) {
    if (!movie.date_added) continue;
    const y = movie.date_added.slice(0, 4);
    addedYears.set(y, (addedYears.get(y) || 0) + 1);
  }
  return {
    totalMovies: scoped.length,
    noCover: scoped.filter((m) => !m.front_cover_url).length,
    noCast: scoped.filter((m) => !(m.cast && m.cast.length)).length,
    genres: tally((m) => m.genres || []),
    studios: tally((m) => m.studios || []),
    formats: tally((m) => m.formats || []),
    countries: tally((m) => (m.country ? [m.country] : [])),
    decades: [...decades.entries()].sort((a, b) => a[0] - b[0]).map(([d, c]) => [`${d}s`, c]),
    addedYears: [...addedYears.entries()].sort((a, b) => a[0].localeCompare(b[0])),
  };
}

function barList(title, entries, limit) {
  const top = limit ? entries.slice(0, limit) : entries;
  const max = top.reduce((m, [, c]) => Math.max(m, c), 1);
  const rows = top.map(([label, count]) => `
    <div class="stat-row">
      <span class="label">${escapeHtml(label)}</span>
      <span class="bar-wrap"><span class="bar" style="width:${((count / max) * 100).toFixed(1)}%"></span></span>
      <span class="val">${count}</span>
    </div>`).join("");
  return `<div class="stat-block"><h3>${escapeHtml(title)}</h3>${rows || `<p class="stat-empty">${t("stats_no_data")}</p>`}</div>`;
}

function renderStats() {
  const s = computeStats();
  document.getElementById("stats-pane").innerHTML = `
    <div class="stats-wrap">
      <h2>${escapeHtml(t("stats_title", { name: collectionLabel(currentCollection) }))}</h2>
      <div class="stat-summary">
        <span>${t("stats_movies", { n: `<strong>${s.totalMovies}</strong>` })}</span>
        <span>${t("stats_no_cover", { n: `<strong>${s.noCover}</strong>` })}</span>
        <span>${t("stats_no_cast", { n: `<strong>${s.noCast}</strong>` })}</span>
      </div>
      <div class="stat-grid">
        ${barList(t("stats_genres"), s.genres, 15)}
        ${barList(t("stats_studios"), s.studios, 10)}
        ${barList(t("stats_formats"), s.formats)}
        ${barList(t("stats_countries"), s.countries)}
        ${barList(t("stats_by_decade"), s.decades)}
        ${barList(t("stats_by_added_year"), s.addedYears)}
      </div>
    </div>`;
}

const statsBtn = document.getElementById("stats-btn");
function setPanel(next) {
  panel = next;
  document.body.classList.toggle("panel-stats", panel === "stats");
  statsBtn.classList.toggle("active", panel === "stats");
  statsBtn.textContent = panel === "stats" ? t("nav_back_to_collection") : t("nav_stats");
  if (panel === "stats") renderStats();
}
statsBtn.addEventListener("click", () => setPanel(panel === "stats" ? "library" : "stats"));

applyStaticLabels();
renderTabs();
renderGroups();
render("");
"##;

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
        assert!(movies.iter().any(|m| !m.discs.is_empty()));
        assert!(movies
            .iter()
            .any(|m| m.discs.iter().any(|d| !d.episodes.is_empty())));
        assert!(movies.iter().any(|m| !m.links.is_empty()));
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
    fn preview_add_movie_flags_new_vs_existing_values_correctly() {
        let Some((scratch, dest)) = fresh_scratch_copy() else {
            return;
        };

        // First add a movie with a known genre/actor, so the preview
        // below has something pre-existing to compare against.
        {
            let conn = open_readonly(&dest).unwrap();
            let plan = build_plan(
                &conn,
                &MovieFields {
                    title: Some("Preview Setup Movie".to_string()),
                    genres: vec!["Preview Existing Genre".to_string()],
                    actors: vec!["Preview Existing Actor".to_string()],
                    ..Default::default()
                },
            )
            .unwrap();
            drop(conn);
            let conn = open_writable(&dest).unwrap();
            commit_plan(&conn, &plan).unwrap();
        }

        let conn = open_readonly(&dest).unwrap();
        let preview = preview_add_movie(
            &conn,
            &MovieFields {
                title: Some("Preview Target Movie".to_string()),
                genres: vec!["Preview Existing Genre".to_string(), "Preview New Genre".to_string()],
                actors: vec!["Preview New Actor".to_string()],
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(preview.title, "Preview Target Movie");
        assert_eq!(preview.new_count, 2);
        let existing_genre = preview.fields.iter().find(|f| f.value == "Preview Existing Genre").unwrap();
        assert!(!existing_genre.is_new);
        let new_genre = preview.fields.iter().find(|f| f.value == "Preview New Genre").unwrap();
        assert!(new_genre.is_new);
        let new_actor = preview.fields.iter().find(|f| f.value == "Preview New Actor").unwrap();
        assert!(new_actor.is_new);

        // Read-only: confirm nothing was actually created by the preview.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ZCOLLECTIBLE WHERE ZTITLE = 'Preview Target Movie'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);

        drop(conn);
        std::fs::remove_dir_all(&scratch).ok();
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
            series: Some("Round Trip Series".to_string()),
            box_set: Some("Round Trip Box Set".to_string()),
            genres: vec!["Round Trip Genre".to_string()],
            studios: vec!["Round Trip Studio".to_string()],
            regions: vec!["Round Trip Region".to_string()],
            screen_ratios: vec!["Round Trip Screen Ratio".to_string()],
            subtitles: vec!["Round Trip Subtitle".to_string()],
            audio_tracks: vec!["Round Trip Audio Track".to_string()],
            hdr: vec!["Round Trip HDR".to_string()],
            extras: vec!["Round Trip Extra".to_string()],
            tags: vec!["Round Trip Tag".to_string()],
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
        assert_eq!(added.series.as_deref(), Some("Round Trip Series"));
        assert_eq!(added.box_set.as_deref(), Some("Round Trip Box Set"));
        assert_eq!(added.genres, vec!["Round Trip Genre".to_string()]);
        assert_eq!(added.studios, vec!["Round Trip Studio".to_string()]);
        assert_eq!(added.regions, vec!["Round Trip Region".to_string()]);
        assert_eq!(added.screen_ratios, vec!["Round Trip Screen Ratio".to_string()]);
        assert_eq!(added.subtitles, vec!["Round Trip Subtitle".to_string()]);
        assert_eq!(added.audio_tracks, vec!["Round Trip Audio Track".to_string()]);
        assert_eq!(added.hdr, vec!["Round Trip HDR".to_string()]);
        assert_eq!(added.extras, vec!["Round Trip Extra".to_string()]);
        assert_eq!(added.tags, vec!["Round Trip Tag".to_string()]);
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
            regions: vec!["Edit Original Region".to_string()],
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
            series: Some("Edit Series".to_string()),
            box_set: Some("Edit Box Set".to_string()),
            genres: vec!["Edit Replacement Genre".to_string()],
            regions: vec!["Edit Replacement Region".to_string()],
            ..Default::default()
        };
        let replace = ReplaceFlags {
            replace_genres: true,
            replace_regions: true,
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
        assert_eq!(edited.regions, vec!["Edit Replacement Region".to_string()]);
        assert_eq!(edited.series.as_deref(), Some("Edit Series"));
        assert_eq!(edited.box_set.as_deref(), Some("Edit Box Set"));

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
                    regions: vec!["Delete Test Region".to_string()],
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
        let orphaned_region_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM Z_7REGIONS WHERE Z_7MOVIES8 = ?1",
                [delete_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(orphaned_region_rows, 0);

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
