# Changelog

> 🇫🇷 [Lire en français](CHANGELOG.fr.md)

History of **versions published on GitHub**, one entry per
tag/release — same convention as [BookNojo's CHANGELOG.md](https://github.com/sergiodadi/BookNojo/blob/main/CHANGELOG.md)
and [MusicNojo's CHANGELOG.md](https://github.com/sergiodadi/MusicNojo/blob/main/CHANGELOG.md).

## [v1.0.0b3] - 2026-11-21

New `.dmg` (`VideoNojo_1.0.0-b3_aarch64.dmg` — the `.dmg` filename now
tracks the published release tag instead of Cargo.toml's per-commit dev
version, fixed in `tauri.conf.json`; previous releases wrongly showed an
internal dev version number in the filename). Also:

- The repository is now **public**.
- README: added a "Screenshots" section (two screenshots of the app)
  right after Installation.
- README: reworded the intro to lead with "Why this project", merging it
  with the opening paragraph.

## [v1.0.0b2] - 2026-11-16

New `.dmg` (`VideoNojo_0.33.0_aarch64.dmg`). Everything added since
v1.0.0b1:

- **Browsing**: A-Z quick-jump bar, sort dropdown and clickable sortable
  column headers, list/grid view toggle (grid shows cover thumbnails),
  pagination, a statistics panel (genres/studios/formats/countries/
  release decade/year added) — in both the application and the static
  HTML export.
- **More fields, read and write**: region, screen ratio, subtitle,
  audio track, HDR, extra, tag; series and box set; cover images
  (`vc_add set-cover`, CLI and GUI).
- **More fields, read-only**: discs and episodes for TV series; links
  (IMDB, trailer, review sites...), clickable, opening the system's
  default browser.
- **Movie detail card**: now shows the cover image (previously only
  the list/grid thumbnails did); genre, studio, format and country are
  clickable, jumping straight to that value's group in the sidebar.
- **Search** now also matches cast member names, not just the title.
- The static HTML export (`vc_export --html`) now has the same
  **EN / FR** toggle as the application — it previously stayed in
  English only.
- `vc_add commit`/`set-cover` gained a `--force` flag, lifting the
  automatic refusal on a path that looks like the real Movie Collector
  collection. A typed `YES` confirmation in the terminal is still
  required before any real write happens.
- A safety check refuses any write while Movie Collector itself is
  currently running.
- `vc_export` gained `--collection`/`--list-collections` to scope a
  CSV/JSON export to one sub-collection.

## [v1.0.0b1] - 2026-11-14

First beta version of VideoNojo, tagged on GitHub, with a
downloadable `.dmg` (`VideoNojo_0.21.0_aarch64.dmg`, unsigned — see
[`README.md`](README.md) for the Gatekeeper "damaged" workaround)
from the very first release.

A macOS application (and `vc_export`/`vc_add` command-line tools) to
view, add, edit and delete movies in a Movie Collector collection —
grouping by genre/studio/format/country, search, resizable panes,
multiple sub-collection tabs, a bilingual French/English interface,
session-state persistence, add/edit-form autocomplete, CSV/JSON/
standalone-HTML export, and a command-line bootstrap for an
independent `.vnc` database.

Day-to-day development history (one feature per commit) lives in the
project's own commit messages (`vX.Y.0: ...`), inside the
repository.
