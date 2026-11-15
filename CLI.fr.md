# Outils en ligne de commande — `vc_export` / `vc_add`

> 🇬🇧 [Read in English](CLI.md)

> Pour un usage quotidien, ces commandes ne sont **pas nécessaires** :
> l'application (voir [`MANUAL.fr.md`](MANUAL.fr.md)) suffit pour
> consulter, ajouter, modifier et supprimer des films. Ce document
> s'adresse à qui veut exporter une collection (CSV/JSON/HTML) ou
> scripter des ajouts depuis le terminal — un usage plus technique et
> facultatif.

## 1. Vue d'ensemble

- **`vc_export`** lit une collection [Movie Collector](https://www.collectorz.com/movie)
  (Collectorz) en lecture seule et produit un export **CSV**, un export
  **JSON**, et/ou une page **HTML** autonome (§ 3).
- **`vc_add`** ajoute, modifie et supprime des films depuis la ligne
  de commande (§ 4), avec les mêmes garde-fous que l'application, et
  amorce une base indépendante à partir d'une collection existante
  (§ 5).

`vc_export` ouvre toujours la base en lecture seule
(`SQLITE_OPEN_READ_ONLY`) : il ne peut jamais modifier la collection
d'origine.

Les commandes ci-dessous supposent que vous êtes dans le dossier
contenant les binaires compilés (`project/target/release/` si compilé
depuis les sources — voir [`BUILD.fr.md`](BUILD.fr.md)).

## 2. `vc_export` — export

```bash
./vc_export \
  --db "~/Documents/Collectorz/Movie Collector/MyCollection.mvcp" \
  --csv movie_collection.csv \
  --json movie_collection.json \
  --html movie_collection.html
```

`--db` accepte soit le dossier `.mvcp`, soit directement le fichier
`Movie_data` qu'il contient. Les trois options de sortie sont
facultatives et indépendantes — ne passer que celles dont on a besoin.

Le **CSV** est un résumé plat, une ligne par film (titre, titre
original, genres/studios/formats joints par `; `, pays, année de
sortie, note). Le **JSON** est la forme imbriquée complète (films →
distribution → équipe technique), les mêmes données que l'application
elle-même charge.

## 3. La page HTML statique

```bash
./vc_export --db "<chemin>" --html movie_collection.html
```

S'ouvre dans n'importe quel navigateur (double-clic, ou `open`), sans
serveur — jaquettes comprises, référencées comme des URL `file://`
locales. Trois zones, comme l'application :

- **à gauche** : regroupement (Genre par défaut, ou Studio/Format/Pays
  via le menu déroulant), avec une barre de raccourci A-Z au-dessus de
  la liste.
- **en haut à droite** : la liste des films du groupe sélectionné —
  recherche, tri (menu déroulant ou en-têtes de colonnes cliquables),
  bascule vue liste/grille, pagination.
- **en bas à droite** : la fiche détail (jaquette, titre, métadonnées,
  distribution et équipe technique).
- un bouton **📊 Statistiques**, pour les mêmes graphiques en barres
  par collection (genres/studios/formats/pays/décennie/année d'ajout)
  que ceux de l'application.

Les onglets en haut changent de sous-collection sans relancer la
commande.

Anglais uniquement (pas de bascule de langue), et il n'y a **pas de
formulaire d'ajout/modification/suppression** — c'est un instantané en
lecture seule, délibérément limité au même périmètre que
l'application elle-même a aujourd'hui. `vc_add` couvre déjà
l'ajout/la modification/la suppression directement et plus sûrement
(un plan à relire avant toute écriture réelle).

## 4. `vc_add` — ajout, modification, suppression

> ⚠️ **Toujours tester sur une copie de sa collection d'abord, jamais
> directement sur l'original.** `vc_add` refuse purement et simplement
> d'écrire sur un chemin qui ressemble à la vraie collection Movie
> Collector (`~/Documents/Collectorz/.../Movie Collector.../`) — voir
> § "Garde-fous" ci-dessous. Il n'y a aucun contournement : toujours
> pointer vers une copie (voir [`BUILD.fr.md`](BUILD.fr.md) pour générer
> des copies de test dédiées).

### Planifier, puis valider

Ajouter, modifier ou supprimer un film se fait en deux temps : une
sous-commande `*-plan` calcule et écrit le SQL qui *serait* exécuté,
sans toucher à la base ; `commit` exécute ensuite le SQL d'un fichier
de plan, dans une seule transaction. Pouvoir relire le plan avant
`commit` est tout l'intérêt : rien n'est écrit avant.

```bash
# 1. Ajout — calcule le plan.
./vc_add add-plan --db "<chemin>" \
  --title "Un Film" \
  --release-year "2024" \
  --country "USA" \
  --genre "Drame" --genre "Thriller" \
  --studio "Un Studio" \
  --format "Blu-ray" \
  --actor "Un Acteur" \
  --crew "Un Réalisateur:Director" \
  --out plan.sql

# 2. Relire plan.sql à la main si on le souhaite.

# 3. Valider — seule cette étape écrit réellement (et sauvegarde d'abord).
./vc_add commit --db "<chemin>" --plan plan.sql
```

`add-plan` accepte chaque champ scalaire (`--title`,
`--title-original`, `--release-year`, `--rating`, `--notes`, `--plot`,
`--upc`, `--country`, `--collection`), et chaque champ répétable
(`--genre`, `--studio`, `--format`, `--actor`, `--crew`, une
occurrence par valeur). `--collection` doit nommer une sous-collection
déjà existante — elle n'est jamais créée. Une entrée `--crew` est soit
`"Nom"`, soit `"Nom:Rôle"` (ex. `"Jane Doe:Director"`) — les rôles
reconnus sont Director, Producer, Writer, Camera, Music, User Credit
1, User Credit 2, et Actor ; un nom de rôle non reconnu est enregistré
sans rôle plutôt que deviné. La date d'ajout du film est horodatée
automatiquement avec la date et l'heure actuelles — il n'y a pas
d'option pour ça, puisque modifier le film par la suite ne la change
jamais non plus.

### Modifier — `edit-plan`

```bash
./vc_add edit-plan --db "<chemin>" --id <id du film> \
  --title "Nouveau Titre" \
  --replace-genres --genre "Nouveau Genre" \
  --out edit.sql
./vc_add commit --db "<chemin>" --plan edit.sql
```

`--id` est le `Z_PK` du film (sortie JSON de `vc_export`, champ
`"id"`). Tous les paramètres scalaire/répétable de `add-plan` sont
disponibles ici aussi, y compris `--collection` (pour déplacer le
film vers une autre sous-collection déjà existante — l'application
elle-même ne propose pas encore cela, voir [`MANUAL.fr.md`](MANUAL.fr.md)).
Un champ répétable n'est **remplacé** que si son option `--replace-*`
correspondante est passée (`--replace-genres`, `--replace-studios`,
`--replace-formats`, `--replace-actors`, `--replace-crew`) — sinon les
valeurs passées pour ce champ sont ignorées et les valeurs existantes
restent inchangées.

### Supprimer — `delete-plan`

```bash
./vc_add delete-plan --id <id du film> --out delete.sql
./vc_add commit --db "<chemin>" --plan delete.sql
```

Supprime le film et tout ce qui lui est spécifique (ses lignes de
liaison distribution/équipe technique/genre/studio/format) — jamais
les lignes de référence partagées qu'il utilisait (un genre, un
studio, un acteur...), puisque d'autres films peuvent encore les
utiliser. Ne touche pas aux disques, épisodes, liens, étiquettes,
région, format d'écran, sous-titres, HDR, bonus ou pistes audio —
rien de tout cela n'est modélisé nulle part encore dans ce projet.

### Garde-fous

Avant d'écrire, `commit` vérifie, dans l'ordre :

1. le chemin de la base **ne ressemble pas** à la vraie collection
   Movie Collector (`~/Documents/Collectorz/...`) — refus sinon, sans
   contournement possible ;
2. une sauvegarde horodatée de `Movie_data` est créée dans
   `backups/`, à côté de la base ;
3. le SQL du plan s'exécute en une seule transaction.

## 5. Pourquoi une base indépendante, séparée de Movie Collector

Movie Collector pour Mac est une application Intel qui dépend de
Rosetta 2 pour tourner sur Apple Silicon. Apple a annoncé la fin
programmée de Rosetta 2 : support complet jusqu'à macOS 27 "Golden
Gate" (automne 2026), largement retiré à partir de macOS 28 (automne
2027). Movie Collector a donc une espérance de vie limitée sur un Mac
à jour, sauf si son éditeur sort une mise à jour native Apple Silicon.

L'objectif à long terme est une base **indépendante** de
l'application — amorcée une fois depuis la vraie collection (§
« `vc_add import` » ci-dessous), puis alimentée uniquement par
VideoNojo, sans plus dépendre de Movie Collector pour grandir.
L'extension choisie pour cette base persistante est **`.vnc`** — elle
fonctionne exactement comme un `.mvcp` (seule la présence d'un fichier
`Movie_data` dedans compte, jamais le nom du dossier). Pour continuer
à alimenter aussi la vraie application Movie Collector pendant une
période de transition, relancer la même séquence `add-plan`/
`edit-plan`/`delete-plan` + `commit` une seconde fois avec `--db`
pointant vers une copie de la vraie collection — chaque base est ainsi
résolue et planifiée **séparément** (jamais le même `plan.sql`
réutilisé sur les deux), puisque les clés primaires et les valeurs de
référence déjà connues divergent dès que les deux bases évoluent
indépendamment.

### `vc_add import` — le transfert de Movie Collector vers VideoNojo

Amorce une nouvelle base indépendante `.vnc` à partir d'une collection
existante (base + dossier `Images/`). Conçu pour n'être exécuté
**qu'une seule fois** : refuse d'écraser une destination qui existe
déjà.

```bash
./vc_add import --source "~/Documents/Collectorz/Movie Collector/MyCollection.mvcp"
# -> collection/MyCollection.vnc (destination par défaut)

# Ou avec une destination explicite :
./vc_add import \
  --source "~/Documents/Collectorz/Movie Collector/MyCollection.mvcp" \
  --dest ~/MyCollectionVideoNojo.vnc
```

## 6. Dépannage

- **`--db "~/Documents/..."` échoue** : si le chemin est entre
  guillemets, le shell n'étend pas lui-même le `~` — l'outil s'en
  charge.
- **Erreur de verrou SQLite** : fermer Movie Collector avant de lancer
  un export ou un ajout.

## 7. Pour aller plus loin

- [`MANUAL.fr.md`](MANUAL.fr.md) : utiliser l'application — suffisant
  pour un usage courant, sans ces commandes.
- [`BUILD.fr.md`](BUILD.fr.md) : compiler les outils et l'application
  depuis les sources.
- [`CHANGELOG.fr.md`](CHANGELOG.fr.md) : historique des versions
  publiées sur GitHub.
- `./vc_export --help` / `./vc_add <sous-commande> --help` : liste
  complète des options.
