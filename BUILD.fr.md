# Compiler VideoNojo depuis les sources

> 🇬🇧 [Read in English](BUILD.md)

Ce document explique comment compiler les outils en ligne de commande
et l'application macOS depuis le code source. Pour **utiliser** le
logiciel une fois compilé, voir [`MANUAL.fr.md`](MANUAL.fr.md)
(l'application — suffisant pour un usage courant) et
[`CLI.fr.md`](CLI.fr.md) (les outils en ligne de commande
`vc_export`/`vc_add`, facultatifs).

## Prérequis

- **Rust** (`cargo`) — installable via Homebrew si besoin :
  `brew install rust`.
- **Tauri CLI**, uniquement pour l'application macOS (pas nécessaire
  pour les seuls outils en ligne de commande) :
  ```bash
  cargo install tauri-cli --version "^2.0.0"
  ```

Tout le code et les manifestes se trouvent sous **`project/`** — la
racine du dépôt ne contient que de la documentation (`README.md`,
`BUILD.md`, `MANUAL.md`, `CLI.md`, `CHANGELOG.md`). Toutes les commandes
ci-dessous supposent que vous êtes dans `project/`.

## Outils en ligne de commande (`vc_export`, `vc_add`)

```bash
cd videonojo/project
cargo build --release
```

Les binaires sont alors `target/release/vc_export` et
`target/release/vc_add` (donc `project/target/release/...` depuis la
racine du dépôt).

```bash
cargo test --release
```

## Application macOS

```bash
cd app
cargo tauri dev      # fenêtre de développement, rechargement à chaud
cargo tauri build    # construit une vraie .app (et un .dmg), dans
                      # app/src-tauri/target/release/bundle/
```

Le frontend de l'app (`app/src/{index.html,app.js,style.css}`) est
édité directement — il n'y a pas d'étape de génération depuis des
templates partagés.

`cargo build --release` (depuis `project/`) ne produit **pas** une
application installable — seul `cargo tauri build` (dans
`project/app/`) le fait.

### Icône de l'application

Les icônes actuellement présentes sous `app/src-tauri/icons/` sont
celles par défaut du squelette Tauri — VideoNojo n'a pas encore
d'icône personnalisée. Une fois une image source 1024×1024 créée, elle
peut être transformée en jeu d'icônes complet avec :

```bash
cd project/app
cargo tauri icon chemin/vers/source.png
```

## Données de test (développement uniquement)

Ces copies et scripts ne sont nécessaires que pour contribuer au
projet — pas pour utiliser le logiciel sur sa propre collection. Depuis
`project/` :

```bash
./refresh-testdata.sh      # testdata/ : copie en lecture seule, pour vc_export
./refresh-testdata-rw.sh   # testdata-rw/ : copie modifiable, pour vc_add
```

`sample-collection/` (commitée dans le dépôt, entièrement fictive —
voir [`MANUAL.fr.md`](MANUAL.fr.md) § 1) peut être régénérée avec :

```bash
cargo run --release --example generate_sample_collection
```

Aucun de ces scripts, ni les outils eux-mêmes, n'écrit jamais dans le
vrai fichier de collection de l'utilisateur
(`~/Documents/Collectorz/Movie Collector/...`) — une règle du projet,
vérifiée par des tests dédiés.

## Structure du dépôt

```
README.md, BUILD.md, MANUAL.md,     documentation (racine du dépôt)
CLI.md, CHANGELOG.md, LICENSE

project/
├─ src/              bibliothèque partagée + binaires CLI (vc_export, vc_add)
├─ app/              application macOS (Tauri) — réutilise src/ via une
│                     dépendance de chemin
├─ examples/          génère la collection d'exemple (sample-collection/)
├─ Cargo.toml         manifeste du crate racine (video_collector_export)
└─ refresh-testdata*.sh   scripts de données de test
```
