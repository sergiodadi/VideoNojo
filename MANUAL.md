# User manual — VideoNojo application

> 🇫🇷 [Lire en français](MANUAL.fr.md)

This document explains how to use the macOS **VideoNojo** application
once installed. To download it, see [`README.md`](README.md); to
compile it from source, see [`BUILD.md`](BUILD.md).

## 1. Opening a collection

- **"Open Collection…"** (in the header, or File → Open Collection…
  in the menu bar) opens the native macOS file picker to choose a
  `.mvcp` collection. The picker only accepts `.mvcp` files.
- A `.mvcp` collection from a Movie Collector version other than the
  one VideoNojo was built against (20.2.2.1) is rejected at this
  point, with a clear message rather than a crash or silent
  corruption.
- On the very first launch, before opening anything, VideoNojo shows
  an entirely fictional bundled sample collection (8 invented movies,
  no personal data) — so there's always something to see, without ever
  exposing real data by default.
- Every later launch reopens the same collection, at the exact same
  spot — active tab, grouping, search, and selected movie are all
  remembered.
- The header always displays the name of the open file, a version
  badge, and an EN/FR language toggle.

## 2. Browsing the library

- **Three zones**: the grouping panel on the left, the movie list at
  top right, the detail card at bottom right. The dividers between the
  three zones can be resized with the mouse.
- **Grouping** by Genre (default), Studio, Format or Country, with an
  A-Z quick-jump bar above the list that narrows it to values starting
  with a given letter. The group list is sorted alphabetically by
  default — clicking the count next to "All" switches it to sort by
  movie count instead (click again to switch back).
- **Search** by title or cast member name.
- **Sort** by title, year, rating, country, or date added (newest
  first), via the dropdown above the list or by clicking a column
  header (clicking it again reverses the direction).
- **List or grid view**: the grid shows cover thumbnails (when
  present) instead of table rows; both are paginated, 60 movies per
  page.
- **Multiple sub-collections** in a single file — tabs at the top
  switch between them, scoping the list and groups to whichever is
  active.
- **Movie detail card**: cover, title, original title, release year,
  country, series, box set, whether it's a TV series, genres, studios, formats,
  regions, screen ratios, subtitles, audio tracks, HDR, extras, tags,
  cast, crew (with role, when known), rating, notes, plot, date added,
  and links (IMDB, trailer, review sites...) — clicking one opens it
  in the system's default browser, not inside the application — plus,
  for a TV series, its discs and their episodes (shown only when a
  disc actually carries something; a bare disc with no episodes and no
  notes of its own isn't shown). Genre, studio, format and country are
  clickable: clicking one jumps straight to the matching group in the
  left panel, filtered to that exact value.
- **Statistics panel** (the "📊 Statistics" button in the header):
  total movie count, how many have no cover or no cast, and bar charts
  of genres, studios, formats, countries, release decade, and year
  added — all scoped to the active sub-collection tab.
- Typing in the add/edit form's country/genre/studio/format/actor/crew
  fields offers autocomplete suggestions drawn from the collection's
  own existing values.

## 3. Adding, editing, deleting a movie

- **Add a movie**: a form covering title, original title, release
  year, rating, country, series, box set, genres, studios, formats,
  regions, screen ratios, subtitles, audio tracks, HDR, extras, tags,
  actors and crew (comma-separated; a crew entry can be `"Name"` or
  `"Name:Role"`), plot and notes. Before anything is written, a
  preview lists every value typed into one of the comma-separated
  fields, with a
  "new" badge on the ones that don't already exist in the collection
  (catching a typo before it becomes a permanent duplicate entry) —
  confirming it is what actually adds the movie. The new movie joins
  whichever sub-collection tab is currently active, and is stamped
  with the current date and time as its date added — editing a movie
  never changes this later.
- **Edit a movie**: from its detail card, the same form opens
  pre-filled with the current values, and writes immediately (no
  preview step).
- **Delete a movie**: from its detail card, after a confirmation
  dialog.
- After any add, edit or delete, the library is automatically
  reloaded: the change appears immediately, with no manual action
  needed.
- A cover can also be picked right from the Add-movie form (a "Choose
  an image…" button, optional): it's attached to the movie once the
  add is confirmed. If the add succeeds but the cover can't be saved,
  the movie is still added — fix the cover afterward from the "no
  cover" list in the Statistics panel, or with `vc_add set-cover`.
- **Setting a cover image**: from the statistics panel, expanding "N
  with no cover" and clicking a movie in that list opens the native
  macOS file picker to choose a front cover image — it's copied into
  the collection's `Images/` folder. There's no way yet to set a
  cover for a movie that already has one, or to set the back cover,
  from the application (`vc_add set-cover` can do both; see
  [`CLI.md`](CLI.md)).

## 4. Bilingual interface

- An **EN / FR** toggle in the header switches the whole interface —
  grouping, list, detail card, add/edit form — between English and
  French. The choice is remembered from one session to the next (and
  so is the chosen layout size).

## 5. Data safety

- The user's real Movie Collector collection is never modified by
  accident: every write (add, edit, delete) is preceded by a
  timestamped backup of the file, and is refused outright — with no
  override reachable from the application — if the path looks like the
  real collection.
- Every write is also refused outright if **Movie Collector itself is
  currently running** — writing to the file while the app holds it
  open risks an inconsistent result, or the app silently overwriting
  the change on its next save. Close Movie Collector first.
- VideoNojo can open the real collection in **read-only** mode to
  browse it (opening it is always `SQLITE_OPEN_READ_ONLY` at the
  database level), but none of the write actions above have any
  exception for it.

## 6. What the application doesn't do yet

- Setting a cover for a movie that already has one, replacing an
  existing cover, or setting a back cover — section 3 above covers
  what *is* possible from the statistics panel.
- Changing a movie's sub-collection from the application — only *add*
  places it in whichever tab is active. The `vc_add` CLI's
  `edit-plan --collection <name>` can move an existing movie to a
  different already-existing sub-collection; see [`CLI.md`](CLI.md).
- Links — read-only (shown on the detail card, clickable, opens in
  the system browser); no way yet to add, edit, or remove one, from
  either the application or `vc_add`. The test collection only has
  online links (IMDB, trailer, review sites) to verify against — a
  local-file link would also be read and shown the same way, but
  that specific case is unverified.
- Discs and episodes — read-only (shown on a movie's detail card when
  a disc actually carries something); no way yet to add, edit, or
  delete an individual disc or episode from either the application or
  `vc_add`.
- Series and box set — read and write (a single value per movie, same
  as country), but nothing yet for browsing *by* series or box set
  (no grouping/tab for either, unlike genre/studio/format), and no way
  to see every movie in a given series or box set at once.
  (Region, screen ratio, subtitle, audio track, HDR, extra, and tag —
  the many-to-many structures — are fully modeled, read and write,
  since v0.25.0.)
- Online lookup to pre-fill a record from a barcode or a database
  match — not started.
- Not yet code-signed/notarized with an Apple developer account — no
  consequence for personal use, but needed before distributing the
  application to anyone else.

## 7. Going further

- [`BUILD.md`](BUILD.md): compiling the application from source.
- [`CHANGELOG.md`](CHANGELOG.md): history of versions published on
  GitHub.

For everyday use, the application alone is enough — you don't need to
know any terminal commands. For the command-line tools
(`vc_export`/`vc_add`), meant for more technical use (CSV/JSON/HTML
export, scripts), see [`CLI.md`](CLI.md).
