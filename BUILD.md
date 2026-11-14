# Compiling VideoNojo from source

> 🇫🇷 [Lire en français](BUILD.fr.md)

This document explains how to compile the command-line tools and the
macOS application from source code. To **use** the software once
compiled, see [`MANUAL.md`](MANUAL.md) (the application — enough for
everyday use) and [`CLI.md`](CLI.md) (the `vc_export`/`vc_add`
command-line tools, optional).

## Prerequisites

- **Rust** (`cargo`) — installable via Homebrew if needed:
  `brew install rust`.
- **Tauri CLI**, only for the macOS application (not needed for the CLI
  tools alone):
  ```bash
  cargo install tauri-cli --version "^2.0.0"
  ```

All the code and manifests live under **`project/`** — the repo root
only contains documentation (`README.md`, `BUILD.md`, `MANUAL.md`,
`CLI.md`, `CHANGELOG.md`). All the commands below assume you are
inside `project/`.

## Command-line tools (`vc_export`, `vc_add`)

```bash
cd videonojo/project
cargo build --release
```

The binaries are then `target/release/vc_export` and
`target/release/vc_add` (so `project/target/release/...` from the repo
root).

```bash
cargo test --release
```

## macOS application

```bash
cd app
cargo tauri dev      # development window, hot reload
cargo tauri build    # builds a real .app (and a .dmg), in
                      # app/src-tauri/target/release/bundle/
```

The app's frontend (`app/src/{index.html,app.js,style.css}`) is edited
directly — there is no shared-template/generation step to run first.

`cargo build --release` (from `project/`) does **not** produce an
installable application — only `cargo tauri build` (inside
`project/app/`) does.

### Application icon

The icons currently bundled under `app/src-tauri/icons/` are the
Tauri scaffold's own defaults — VideoNojo has no custom icon yet. Once
a 1024×1024 source image exists, it can be turned into the full icon
set with:

```bash
cd project/app
cargo tauri icon path/to/source.png
```

## Test data (development only)

These copies and scripts are only needed to contribute to the project —
not to use the software on your own collection. From `project/`:

```bash
./refresh-testdata.sh      # testdata/: read-only copy, for vc_export
./refresh-testdata-rw.sh   # testdata-rw/: writable copy, for vc_add
```

`sample-collection/` (committed to the repo, entirely fictional — see
[`MANUAL.md`](MANUAL.md) § 1) can be regenerated with:

```bash
cargo run --release --example generate_sample_collection
```

None of these scripts, nor the tools themselves, ever write to the
user's real collection file (`~/Documents/Collectorz/Movie Collector/...`)
— a project rule, checked by dedicated tests.

## Repository structure

```
README.md, BUILD.md, MANUAL.md,     documentation (repo root)
CLI.md, CHANGELOG.md, LICENSE

project/
├─ src/              shared library + CLI binaries (vc_export, vc_add)
├─ app/              macOS application (Tauri) — reuses src/ via a path
│                     dependency
├─ examples/          generates the sample collection (sample-collection/)
├─ Cargo.toml         root crate manifest (video_collector_export)
└─ refresh-testdata*.sh   test-data scripts
```
