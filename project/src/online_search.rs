//! Online movie/TV-show metadata lookup ("Recherche en ligne" feature) —
//! two independent, free, keyless sources: Wikidata (for films) and
//! TVmaze (for TV shows). Each source's function is self-contained and
//! returns `Err` only for a genuine transport/parse failure; a source
//! simply having zero matches returns `Ok(vec![])`, never an error — the
//! caller (a Tauri command) runs both and shows whatever comes back,
//! without one failing source blocking the other.
//!
//! Unlike BookNojo's three book sources, no free, keyless source exists
//! for film cover art (TMDB/OMDb both require signing up for an API key,
//! which would break this app family's "no account needed" convention;
//! Apple's own iTunes Search API no longer indexes movies at all for
//! `entity=movie` — confirmed empirically, 0 results for every title
//! tried, in every country code). Wikidata's `cover_url` is therefore
//! always `None` for films — poster images aren't on Wikimedia Commons
//! either, being copyrighted, unlike book covers or album art. TV shows
//! fare better: TVmaze does provide a real poster image.

use serde::Serialize;

/// One candidate movie or TV show, normalized from whichever source
/// returned it. `source` is a short label ("Wikidata"/"TVmaze"), shown to
/// the user so they know which result they're picking — results are
/// deliberately never merged across sources.
#[derive(Debug, Clone, Serialize)]
pub struct MovieSearchResult {
    pub source: &'static str,
    pub title: String,
    pub director: Option<String>,
    pub genres: Vec<String>,
    pub release_year: Option<String>,
    pub cover_url: Option<String>,
}

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent("VideoNojo (https://github.com/sergiodadi/VideoNojo)")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())
}

/// Wikidata's time claims come back as e.g. `"+2014-10-26T00:00:00Z"` —
/// strips the leading sign and takes the 4-digit year.
fn extract_wikidata_year(time: &str) -> Option<String> {
    let digits = time.trim_start_matches(['+', '-']);
    digits.get(0..4).map(str::to_string)
}

fn qid_from_claim(claim: &serde_json::Value) -> Option<String> {
    claim
        .get("mainsnak")
        .and_then(|m| m.get("datavalue"))
        .and_then(|d| d.get("value"))
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

// ---------------------------------------------------------------------
// Wikidata — films
// ---------------------------------------------------------------------

/// `wikidata.org/w/api.php` — keyless, no published hard rate limit. Two
/// round trips, deliberately not SPARQL (tried first; the federated
/// `wikibase:mwapi`/`EntitySearch` query needed to do a free-text search
/// in SPARQL came back with ranking that didn't even put an exact-title
/// match first, and with duplicate cross-product rows per claim — too
/// unreliable for this):
///
/// 1. `wbsearchentities` (free-text search) — each candidate's own
///    `description` is used as a cheap but effective disambiguator: only
///    candidates whose English description contains "film" are kept
///    (confirmed necessary: searching "Interstellar" alone also surfaces
///    the interstellar medium, a video game, etc. — description is the
///    reliable signal, title alone is not).
/// 2. `wbgetentities` on the surviving candidates (max 5) for their
///    claims — `P57` (director), `P136` (genre), `P577` (publication
///    date). These claims hold *entity references* (Q-ids), not labels
///    directly, so a 3rd batched `wbgetentities` call resolves every
///    referenced director/genre id to its English label in one request.
pub fn search_wikidata_film(query: &str) -> Result<Vec<MovieSearchResult>, String> {
    let client = http_client()?;

    let search_json: serde_json::Value = client
        .get("https://www.wikidata.org/w/api.php")
        .query(&[
            ("action", "wbsearchentities"),
            ("search", query),
            ("language", "en"),
            ("format", "json"),
            ("limit", "10"),
            ("type", "item"),
        ])
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;

    let candidate_ids: Vec<String> = search_json
        .get("search")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let id = item.get("id")?.as_str()?.to_string();
                    let description = item
                        .get("display")
                        .and_then(|d| d.get("description"))
                        .and_then(|d| d.get("value"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    description.to_lowercase().contains("film").then_some(id)
                })
                .take(5)
                .collect()
        })
        .unwrap_or_default();

    if candidate_ids.is_empty() {
        return Ok(vec![]);
    }

    let entities = wbgetentities(&client, &candidate_ids, "claims|labels")?;

    // Every director/genre id referenced by any candidate, deduplicated,
    // resolved to an English label in one more batched call.
    let mut referenced_ids: Vec<String> = Vec::new();
    for entity in entities.values() {
        let Some(claims) = entity.get("claims") else { continue };
        for prop in ["P57", "P136"] {
            if let Some(arr) = claims.get(prop).and_then(|v| v.as_array()) {
                for claim in arr {
                    if let Some(id) = qid_from_claim(claim) {
                        if !referenced_ids.contains(&id) {
                            referenced_ids.push(id);
                        }
                    }
                }
            }
        }
    }
    let labels = if referenced_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        let label_entities = wbgetentities(&client, &referenced_ids, "labels")?;
        label_entities
            .into_iter()
            .filter_map(|(id, entity)| {
                entity
                    .get("labels")
                    .and_then(|l| l.get("en"))
                    .and_then(|l| l.get("value"))
                    .and_then(|v| v.as_str())
                    .map(|label| (id, label.to_string()))
            })
            .collect()
    };

    Ok(candidate_ids
        .iter()
        .filter_map(|id| wikidata_result_from_entity(entities.get(id)?, &labels))
        .collect())
}

/// Issues one `wbgetentities` call for up to 50 ids (comfortably above
/// what this module ever requests in a single batch) and returns the
/// `entities` object as an id→entity map.
fn wbgetentities(
    client: &reqwest::blocking::Client,
    ids: &[String],
    props: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let ids_param = ids.join("|");
    let json: serde_json::Value = client
        .get("https://www.wikidata.org/w/api.php")
        .query(&[
            ("action", "wbgetentities"),
            ("ids", ids_param.as_str()),
            ("props", props),
            ("languages", "en"),
            // Many items (people, single-word/eponymous titles) only
            // carry a language-neutral "mul" label, not a dedicated "en"
            // one — confirmed directly: Christopher Nolan's own item has
            // no "en" label, only "mul", and comes back empty without
            // this. `languagefallback` makes the API resolve "mul" (and
            // the rest of the normal fallback chain) into the requested
            // language's slot instead of leaving it empty.
            ("languagefallback", "1"),
            ("format", "json"),
        ])
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;
    Ok(json
        .get("entities")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default())
}

fn wikidata_result_from_entity(
    entity: &serde_json::Value,
    labels: &std::collections::HashMap<String, String>,
) -> Option<MovieSearchResult> {
    let title = entity
        .get("labels")
        .and_then(|l| l.get("en"))
        .and_then(|l| l.get("value"))
        .and_then(|v| v.as_str())?
        .to_string();
    let claims = entity.get("claims");

    let director = claims
        .and_then(|c| c.get("P57"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(qid_from_claim)
        .and_then(|qid| labels.get(&qid).cloned());

    let genres = claims
        .and_then(|c| c.get("P136"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(qid_from_claim)
                .filter_map(|qid| labels.get(&qid).cloned())
                .collect()
        })
        .unwrap_or_default();

    // Several P577 claims (theatrical/regional release dates) are common;
    // the earliest one is used — ISO-shaped Wikidata time strings
    // ("+YYYY-MM-DD...") sort lexically the same as chronologically.
    let release_year = claims
        .and_then(|c| c.get("P577"))
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .filter_map(|c| {
                    c.get("mainsnak")
                        .and_then(|m| m.get("datavalue"))
                        .and_then(|d| d.get("value"))
                        .and_then(|v| v.get("time"))
                        .and_then(|v| v.as_str())
                })
                .min()
        })
        .and_then(extract_wikidata_year);

    Some(MovieSearchResult {
        source: "Wikidata",
        title,
        director,
        genres,
        release_year,
        // Never set: film posters are copyrighted, so practically never
        // present on Wikimedia Commons (confirmed: P18 is absent even on
        // a famous, heavily-edited film item). See this module's doc
        // comment.
        cover_url: None,
    })
}

// ---------------------------------------------------------------------
// TVmaze — TV shows
// ---------------------------------------------------------------------

/// `api.tvmaze.com` — keyless, no authentication, one request per query.
/// Unlike Wikidata, its results do carry a usable poster image.
pub fn search_tvmaze_show(query: &str) -> Result<Vec<MovieSearchResult>, String> {
    let client = http_client()?;
    let json: serde_json::Value = client
        .get("https://api.tvmaze.com/search/shows")
        .query(&[("q", query)])
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;
    let items = json.as_array().cloned().unwrap_or_default();
    Ok(items.iter().filter_map(tvmaze_result_from_item).take(10).collect())
}

fn tvmaze_result_from_item(item: &serde_json::Value) -> Option<MovieSearchResult> {
    let show = item.get("show")?;
    let title = show.get("name").and_then(|v| v.as_str())?.to_string();
    let genres = show
        .get("genres")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|g| g.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    let release_year = show
        .get("premiered")
        .and_then(|v| v.as_str())
        .and_then(|s| s.get(0..4))
        .map(str::to_string);
    let cover_url = show
        .get("image")
        .and_then(|img| img.get("medium"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Some(MovieSearchResult {
        source: "TVmaze",
        title,
        director: None,
        genres,
        release_year,
        cover_url,
    })
}

// ---------------------------------------------------------------------
// Cover download
// ---------------------------------------------------------------------

/// Counter making each downloaded cover's temp filename unique within
/// this run — same reasoning as BookNojo's own `ONLINE_COVER_COUNTER`.
static ONLINE_COVER_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Downloads a chosen online search result's `cover_url` (only ever set
/// for a TVmaze result — see this module's doc comment) into a throwaway
/// file under the system temp dir, and returns its local path, meant to
/// be handed to `set_cover` exactly like a user's own manually-picked
/// file. Never cleaned up here: the OS temp dir clears on its own
/// schedule, and the add-time cover mechanism only *reads* the path.
pub fn download_cover_to_temp_file(url: &str) -> Result<String, String> {
    let client = http_client()?;
    let resp = client.get(url).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Cover download failed ({}).", resp.status()));
    }
    let extension = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(';').next().unwrap_or(v).trim().to_string())
        .map(|content_type| match content_type.as_str() {
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "jpg",
        })
        .unwrap_or("jpg");
    let bytes = resp.bytes().map_err(|e| e.to_string())?;

    let n = ONLINE_COVER_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut path = std::env::temp_dir();
    path.push(format!("videonojo-online-cover-{}-{n}.{extension}", std::process::id()));
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_wikidata_year_strips_sign_and_takes_first_4_digits() {
        assert_eq!(extract_wikidata_year("+2014-10-26T00:00:00Z"), Some("2014".to_string()));
        assert_eq!(extract_wikidata_year("-0044-01-01T00:00:00Z"), Some("0044".to_string()));
        assert_eq!(extract_wikidata_year("x"), None);
    }

    #[test]
    fn qid_from_claim_extracts_the_referenced_entity_id() {
        let claim: serde_json::Value = serde_json::from_str(
            r#"{"mainsnak": {"datavalue": {"value": {"entity-type": "item", "id": "Q25191"}}}}"#,
        )
        .unwrap();
        assert_eq!(qid_from_claim(&claim), Some("Q25191".to_string()));
    }

    #[test]
    fn qid_from_claim_is_none_for_a_non_entity_value() {
        let claim: serde_json::Value =
            serde_json::from_str(r#"{"mainsnak": {"datavalue": {"value": "plain string"}}}"#).unwrap();
        assert_eq!(qid_from_claim(&claim), None);
    }

    /// A hand-written fixture matching what `wbgetentities` returns for
    /// a real film item (trimmed: Interstellar, Q13417189) — exercises
    /// label resolution independent of live network access.
    #[test]
    fn wikidata_result_from_entity_resolves_director_and_genre_labels() {
        let entity: serde_json::Value = serde_json::from_str(
            r#"{
                "labels": {"en": {"language": "en", "value": "Interstellar"}},
                "claims": {
                    "P57": [{"mainsnak": {"datavalue": {"value": {"id": "Q25191"}}}}],
                    "P136": [
                        {"mainsnak": {"datavalue": {"value": {"id": "Q471839"}}}},
                        {"mainsnak": {"datavalue": {"value": {"id": "Q319221"}}}}
                    ],
                    "P577": [{"mainsnak": {"datavalue": {"value": {"time": "+2014-11-07T00:00:00Z"}}}}]
                }
            }"#,
        )
        .unwrap();
        let mut labels = std::collections::HashMap::new();
        labels.insert("Q25191".to_string(), "Christopher Nolan".to_string());
        labels.insert("Q471839".to_string(), "science fiction film".to_string());
        labels.insert("Q319221".to_string(), "drama film".to_string());

        let result = wikidata_result_from_entity(&entity, &labels).unwrap();
        assert_eq!(result.source, "Wikidata");
        assert_eq!(result.title, "Interstellar");
        assert_eq!(result.director, Some("Christopher Nolan".to_string()));
        assert_eq!(
            result.genres,
            vec!["science fiction film".to_string(), "drama film".to_string()]
        );
        assert_eq!(result.release_year, Some("2014".to_string()));
        assert_eq!(result.cover_url, None);
    }

    #[test]
    fn wikidata_result_from_entity_is_none_without_an_english_label() {
        let entity: serde_json::Value = serde_json::from_str(r#"{"labels": {}, "claims": {}}"#).unwrap();
        let labels = std::collections::HashMap::new();
        assert!(wikidata_result_from_entity(&entity, &labels).is_none());
    }

    #[test]
    fn wikidata_result_from_entity_tolerates_entirely_missing_claims() {
        let entity: serde_json::Value =
            serde_json::from_str(r#"{"labels": {"en": {"value": "Untitled"}}}"#).unwrap();
        let labels = std::collections::HashMap::new();
        let result = wikidata_result_from_entity(&entity, &labels).unwrap();
        assert_eq!(result.title, "Untitled");
        assert_eq!(result.director, None);
        assert!(result.genres.is_empty());
        assert_eq!(result.release_year, None);
    }

    /// A hand-written fixture matching TVmaze's real response shape for
    /// a single search hit.
    #[test]
    fn tvmaze_result_from_item_parses_a_real_shaped_response() {
        let item: serde_json::Value = serde_json::from_str(
            r#"{
                "show": {
                    "name": "Breaking Bad",
                    "genres": ["Drama", "Crime", "Thriller"],
                    "premiered": "2008-01-20",
                    "image": {"medium": "https://static.tvmaze.com/uploads/images/medium_portrait/501/1253519.jpg"}
                }
            }"#,
        )
        .unwrap();
        let result = tvmaze_result_from_item(&item).unwrap();
        assert_eq!(result.source, "TVmaze");
        assert_eq!(result.title, "Breaking Bad");
        assert_eq!(result.genres, vec!["Drama".to_string(), "Crime".to_string(), "Thriller".to_string()]);
        assert_eq!(result.release_year, Some("2008".to_string()));
        assert!(result.cover_url.unwrap().contains("1253519"));
    }

    #[test]
    fn tvmaze_result_from_item_is_none_without_a_name() {
        let item: serde_json::Value = serde_json::from_str(r#"{"show": {}}"#).unwrap();
        assert!(tvmaze_result_from_item(&item).is_none());
    }

    #[test]
    fn tvmaze_result_from_item_tolerates_a_missing_image() {
        let item: serde_json::Value =
            serde_json::from_str(r#"{"show": {"name": "No Poster Show"}}"#).unwrap();
        let result = tvmaze_result_from_item(&item).unwrap();
        assert_eq!(result.cover_url, None);
    }

    // Live network tests — real calls to the actual services, not mocks.

    #[test]
    fn search_wikidata_film_finds_interstellar_with_description_filter() {
        let results = search_wikidata_film("Interstellar").expect("network call failed");
        let interstellar = results
            .iter()
            .find(|r| r.title == "Interstellar")
            .expect("should find the 2014 Christopher Nolan film among the results");
        assert_eq!(interstellar.director, Some("Christopher Nolan".to_string()));
        assert_eq!(interstellar.release_year, Some("2014".to_string()));
    }

    #[test]
    fn search_tvmaze_show_finds_breaking_bad() {
        let results = search_tvmaze_show("Breaking Bad").expect("network call failed");
        assert!(results.iter().any(|r| r.title == "Breaking Bad"));
    }

    #[test]
    fn download_cover_to_temp_file_saves_a_real_tvmaze_cover() {
        let path = download_cover_to_temp_file(
            "https://static.tvmaze.com/uploads/images/medium_portrait/501/1253519.jpg",
        )
        .expect("download failed");
        let metadata = std::fs::metadata(&path).expect("downloaded file should exist");
        assert!(metadata.len() > 1000, "downloaded cover is suspiciously small: {} bytes", metadata.len());
        assert!(path.ends_with(".jpg"), "expected a .jpg extension from the jpeg content-type, got: {path}");
        std::fs::remove_file(&path).ok();
    }
}
