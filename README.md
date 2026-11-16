# VideoNojo

> 🇫🇷 [Lire en français](README.fr.md)

## Why this project

Movie collection manager. Movie Collector for Mac is an Intel app
that depends on Rosetta 2. Apple has announced the planned end of
Rosetta 2 (full support only through macOS 27, largely removed
starting with macOS 28, ~fall 2027) — so Movie Collector has a
limited remaining lifespan on an up-to-date Mac. VideoNojo imports a
[Movie Collector](https://www.collectorz.com/movie) (Collectorz)
collection's database into its own tools — reading and writing a copy
of the existing collection, with export, browse, add, edit, and
delete — so it keeps working, and keeps growing, without being
limited to what the vendor app itself offers, or depending on Movie
Collector for either, the day it no longer runs.

VideoNojo is a sibling project to [BookNojo](https://github.com/sergiodadi/BookNojo)
and [MusicNojo](https://github.com/sergiodadi/MusicNojo) (the same
approach and the same underlying Rosetta 2 problem, applied to Book
Collector and Music Collector respectively).

> Private project — the GitHub repository isn't public yet.

## What it does today

- **macOS application** (icon, window, native menus) to view, add, edit
  and delete movies in a collection — with automatic backups before
  any write, and an outright refusal to write to anything that looks
  like your real collection.
- Grouping by genre/studio/format/country; search; resizable panes.
- Multiple sub-collections in one file — tabs switch between them.
- Bilingual French/English interface.
- `<datalist>` autocomplete on the add/edit form, from the collection's
  own existing values.
- Command-line export to **CSV**, **JSON**, and a standalone **HTML**
  page; a command-line bootstrap to start an independent database from
  an existing collection.

➡️ Full guide to using the application: [`MANUAL.md`](MANUAL.md).
Command-line tools also exist (CSV/JSON/HTML export, scripting) —
optional, not needed for everyday use: see [`CLI.md`](CLI.md).

## Installation

Download the `.dmg` from the [latest release](https://github.com/sergiodadi/VideoNojo/releases/latest),
open it, and drag VideoNojo.app into Applications.

> ⚠️ **"VideoNojo is damaged and can't be opened"?** VideoNojo isn't
> signed with an Apple Developer certificate, so macOS Gatekeeper
> quarantines it and shows this misleading message — the app isn't
> actually damaged. Fix it by running this once in Terminal, then open
> the app normally:
> ```bash
> xattr -cr /Applications/VideoNojo.app
> ```
> If that doesn't work, right-click (or Control-click) the app and
> choose **Open** instead of double-clicking.

To compile from source instead of using the `.dmg`, see
[`BUILD.md`](BUILD.md).

> Tested only on **macOS Tahoe (26)** — not verified on other macOS
> versions.

## Version history

See [`CHANGELOG.md`](CHANGELOG.md).

## License

[MIT](LICENSE).

---

These packages were built, tested and published with the help of Claude, the AI assistant by Anthropic.

Movie Collector and Collectorz/CLZ are trademarks of their respective owners. VideoNojo is an independent project, not affiliated with these companies.
