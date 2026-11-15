# Command-line tools — `vc_export` / `vc_add`

> 🇫🇷 [Lire en français](CLI.fr.md)

> For everyday use, these commands are **not needed**: the application
> (see [`MANUAL.md`](MANUAL.md)) is enough to view, add, edit and
> delete movies. This document is for anyone who wants to export a
> collection (CSV/JSON/HTML) or script additions from the terminal — a
> more technical, optional use.

## 1. Overview

- **`vc_export`** reads a [Movie Collector](https://www.collectorz.com/movie)
  (Collectorz) collection read-only and produces a **CSV** export, a
  **JSON** export, and/or a standalone **HTML page** (§ 3).
- **`vc_add`** adds, edits and deletes movies from the command line
  (§ 4), with the same safety gates as the application, and bootstraps
  an independent database from an existing collection (§ 5).

`vc_export` always opens the database read-only
(`SQLITE_OPEN_READ_ONLY`): it can never modify the original collection.

The commands below assume you are in the folder containing the compiled
binaries (`project/target/release/` if compiled from source — see
[`BUILD.md`](BUILD.md)).

## 2. `vc_export` — export

```bash
./vc_export \
  --db "~/Documents/Collectorz/Movie Collector/MyCollection.mvcp" \
  --csv movie_collection.csv \
  --json movie_collection.json \
  --html movie_collection.html
```

`--db` accepts either the `.mvcp` folder, or directly the `Movie_data`
file it contains. All three output flags are optional and independent —
pass only the ones you need.

The **CSV** is a flat, one-row-per-movie summary (title, title
original, genres/studios/formats joined with `; `, country, release
year, rating). The **JSON** is the full nested shape (movies → cast →
crew), the same data the application itself loads.

A `.mvcp` file can hold several sub-collections. `--list-collections`
prints each one's name and movie count and stops (no CSV/JSON/HTML is
written):

```bash
./vc_export --db "<path>" --list-collections
```

`--collection "<name>"` scopes CSV/JSON/HTML export to just that
sub-collection — without it, every sub-collection is exported together:

```bash
./vc_export --db "<path>" --collection "My movie collection" --json movie_collection.json
```

## 3. The static HTML page

```bash
./vc_export --db "<path>" --html movie_collection.html
```

Opens in any browser (double-click, or `open`), no server needed —
including cover images, referenced as local `file://` URLs. Three
zones, like the application:

- **on the left**: grouping (Genre by default, or Studio/Format/
  Country via the dropdown menu), with an A-Z quick-jump bar above the
  list.
- **top right**: the list of movies in the selected group — search,
  sort (dropdown or clickable column headers), list/grid view toggle,
  pagination.
- **bottom right**: the detail card (cover, title, metadata, cast and
  crew, links, plus discs/episodes for a TV series).
- a **📊 Statistics** button, for the same per-collection bar charts
  (genres/studios/formats/countries/decade/year added) the live
  application has.

The tabs at the top switch sub-collection without re-running the
command.

English only (no language toggle), and there is **no add/edit/delete
form** — it's a read-only snapshot, deliberately scoped to the same
feature set the live application has today. `vc_add` already covers
adding/editing/deleting directly and more safely (a reviewable plan
before any real write).

## 4. `vc_add` — adding, editing, deleting

> ⚠️ **Always test on a copy of your collection first, never directly
> on the original.** `vc_add` refuses outright to write to a path that
> looks like the real Movie Collector collection
> (`~/Documents/Collectorz/.../Movie Collector.../`) — see § "Safety
> gates" below. There is no override: point it at a copy, always (see
> [`BUILD.md`](BUILD.md) for how to generate dedicated test copies).

### Plan, then commit

Adding, editing or deleting a movie is a two-step process: a
`*-plan` subcommand computes and writes the SQL that *would* run,
without touching the database; `commit` then executes a plan file's
SQL, inside a single transaction. Reviewing the plan before `commit`
is the point — nothing is written until you do.

```bash
# 1. Add — compute the plan.
./vc_add add-plan --db "<path>" \
  --title "Some Movie" \
  --release-year "2024" \
  --country "USA" \
  --genre "Drama" --genre "Thriller" \
  --studio "Some Studio" \
  --format "Blu-ray" \
  --actor "Some Actor" \
  --crew "Some Director:Director" \
  --out plan.sql

# 2. Review plan.sql by hand if you want to.

# 3. Commit — this is the only step that actually writes (and backs up first).
./vc_add commit --db "<path>" --plan plan.sql
```

`add-plan` accepts every scalar field (`--title`, `--title-original`,
`--release-year`, `--rating`, `--notes`, `--plot`, `--upc`,
`--country`, `--collection`, `--series`, `--box-set`), and every repeatable field (`--genre`,
`--studio`, `--format`, `--region`, `--screen-ratio`, `--subtitle`,
`--audio-track`, `--hdr`, `--extra`, `--tag`, `--actor`, `--crew`, one
occurrence per value). `--collection` must name an already-existing sub-collection —
it is never created. A `--crew` entry is either `"Name"` or
`"Name:Role"` (e.g. `"Jane Doe:Director"`) — known roles are Director,
Producer, Writer, Camera, Music, User Credit 1, User Credit 2, and
Actor; an unrecognized role name is stored with no role at all rather
than guessed. The movie's date-added is stamped automatically with the
current date and time — there is no flag for it, since editing a movie
never changes it afterward either.

### Editing — `edit-plan`

```bash
./vc_add edit-plan --db "<path>" --id <movie id> \
  --title "New Title" \
  --replace-genres --genre "New Genre" \
  --out edit.sql
./vc_add commit --db "<path>" --plan edit.sql
```

`--id` is the movie's `Z_PK` (`vc_export`'s JSON output, field `"id"`).
Every scalar/repeatable flag from `add-plan` is available here too,
including `--collection` (to move the movie to a different
already-existing sub-collection — the application itself doesn't
expose this yet, see [`MANUAL.md`](MANUAL.md)). A repeatable field is
only **replaced** if you pass its matching `--replace-*` flag
(`--replace-genres`, `--replace-studios`, `--replace-formats`,
`--replace-regions`, `--replace-screen-ratios`, `--replace-subtitles`,
`--replace-audio-tracks`, `--replace-hdr`, `--replace-extras`,
`--replace-tags`, `--replace-actors`, `--replace-crew`) — otherwise
the values you pass are ignored for that field and the existing ones
are left untouched.

### Deleting — `delete-plan`

```bash
./vc_add delete-plan --id <movie id> --out delete.sql
./vc_add commit --db "<path>" --plan delete.sql
```

Removes the movie and everything specific to it (its cast/crew/genre/
studio/format/region/screen-ratio/subtitle/audio-track/HDR/extra/tag
link rows) — never the shared lookup rows it referenced (a genre, a
studio, an actor...), since other movies may still use them. Doesn't
touch discs, episodes, box sets, series or links — none of that is
modeled anywhere in this project yet.

### Covers — `set-cover`

```bash
./vc_add set-cover --db "<path>" --id <movie id> --front cover.jpg
./vc_add set-cover --db "<path>" --id <movie id> --front front.jpg --back back.jpg
```

Writes immediately — unlike the plan/commit split above, a cover is
just a file copy into the package's `Images/` folder (named
`"<id>_f_<slug>.<ext>"` / `"<id>_b_<slug>.<ext>"`, matching the
application's own naming) plus one column each, nothing to review
first.

### Safety gates

Before writing, `commit` and `set-cover` both check, in order:

1. the database path **does not look like** the real Movie Collector
   collection (`~/Documents/Collectorz/...`) — refusal otherwise, with
   no override;
2. **Movie Collector itself is not currently running** (checked via
   `pgrep`) — refusal otherwise, since writing to `Movie_data` while
   the app holds it open risks an inconsistent WAL, or the app
   silently overwriting the change on its next save. Close the app
   first, then retry;
3. a timestamped backup of `Movie_data` is created in `backups/`, next
   to the database;
4. the plan's SQL runs in a single transaction (`commit`) or the file
   copy plus column updates happen (`set-cover`).

## 5. Why an independent database, separate from Movie Collector

Movie Collector for Mac is an Intel app that depends on Rosetta 2 to
run on Apple Silicon. Apple has announced the planned end of Rosetta
2: full support only through macOS 27 "Golden Gate" (fall 2026),
largely removed starting with macOS 28 (fall 2027). Movie Collector
therefore has a limited remaining lifespan on an up-to-date Mac,
unless its publisher ships a native Apple Silicon update.

The long-term goal is a database **independent** from the
application — bootstrapped once from the real collection (§ "`vc_add
import`" below), then fed only by VideoNojo, never depending on Movie
Collector to keep growing. The chosen extension for this persistent
database is **`.vnc`** — it works exactly like a `.mvcp` (only the
presence of a `Movie_data` file inside matters, never the folder's
name). To keep feeding the real Movie Collector application too during
a transition period, run the same `add-plan`/`edit-plan`/`delete-plan`
+ `commit` sequence a second time with `--db` pointing at a copy of
the real collection instead — each database is resolved and planned
**separately** this way (never the same `plan.sql` reused against
both), since primary keys and already-known lookup values diverge as
soon as two databases evolve independently.

### `vc_add import` — the Movie Collector to VideoNojo transfer

Bootstraps a new independent `.vnc` database from an existing
collection (database + `Images/` folder). Only meant to be run
**once**: refuses to overwrite a destination that already exists.

```bash
./vc_add import --source "~/Documents/Collectorz/Movie Collector/MyCollection.mvcp"
# -> collection/MyCollection.vnc (default destination)

# Or with an explicit destination:
./vc_add import \
  --source "~/Documents/Collectorz/Movie Collector/MyCollection.mvcp" \
  --dest ~/MyCollectionVideoNojo.vnc
```

## 6. Troubleshooting

- **`--db "~/Documents/..."` fails**: if the path is quoted, the shell
  doesn't expand `~` itself — the tool does it for you.
- **SQLite lock error**: close Movie Collector before running an export
  or an add.

## 7. Going further

- [`MANUAL.md`](MANUAL.md): using the application — enough for everyday
  use, without these commands.
- [`BUILD.md`](BUILD.md): compiling the tools and the application from
  source.
- [`CHANGELOG.md`](CHANGELOG.md): history of versions published on
  GitHub.
- `./vc_export --help` / `./vc_add <subcommand> --help`: full list of
  options.
