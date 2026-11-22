# Changelog

> 🇬🇧 [Read in English](CHANGELOG.md)

Historique des **versions publiées sur GitHub**, une entrée par
tag/release — même convention que le [CHANGELOG.md de BookNojo](https://github.com/sergiodadi/BookNojo/blob/main/CHANGELOG.md)
et le [CHANGELOG.md de MusicNojo](https://github.com/sergiodadi/MusicNojo/blob/main/CHANGELOG.md).

## [v1.0.0b6] - 2026-11-22

- « VideoNojo Help > About VideoNojo » (une simple boîte nom+version
  qui dupliquait le vrai panneau natif « About ») remplacé par une
  vraie fenêtre d'aide bilingue « User Guide ».

## [v1.0.0b5] - 2026-11-22

- Le panneau détail est désormais une carte sombre (comme dans BookNojo et
  MusicNojo), au lieu d'un fond clair — le titre et les « Liens » sont
  maintenant lisibles sur le nouveau fond sombre.

## [v1.0.0b4] - 2026-11-22

- Nouvelle icône d'application : un carré plat en dégradé orange-rouge
  avec un monogramme blanc « VN », à la place de l'icône générique par
  défaut de Tauri utilisée jusqu'ici.

## [v1.0.0b3] - 2026-11-21

Nouveau `.dmg` (`VideoNojo_1.0.0-b3_aarch64.dmg` — le nom du `.dmg` suit
désormais le tag de release publié au lieu de la version de dev par
commit de Cargo.toml, corrigé dans `tauri.conf.json` ; les releases
précédentes affichaient à tort un numéro de version de dev interne dans
le nom du fichier). Également :

- Le dépôt est désormais **public**.
- README : ajout d'une section « Copie d'écran » (deux captures de
  l'app) juste après Installation.
- README : reformulation de l'intro pour commencer par « Pourquoi ce
  projet », fusionnée avec le paragraphe d'ouverture.

## [v1.0.0b2] - 2026-11-16

Nouveau `.dmg` (`VideoNojo_0.33.0_aarch64.dmg`). Tout ce qui a été
ajouté depuis v1.0.0b1 :

- **Navigation** : barre de saut rapide A-Z, menu déroulant de tri et
  en-têtes de colonnes cliquables/triables, bascule vue liste/grille
  (la grille affiche les vignettes de jaquette), pagination, un
  panneau de statistiques (genres/studios/formats/pays/décennie de
  sortie/année d'ajout) — dans l'application et dans l'export HTML
  statique.
- **Plus de champs, lecture et écriture** : région, format d'écran,
  sous-titre, piste audio, HDR, bonus, étiquette ; série et coffret ;
  jaquettes (`vc_add set-cover`, CLI et interface).
- **Plus de champs, lecture seule** : disques et épisodes pour les
  séries TV ; liens (IMDB, bande-annonce, sites de critiques…),
  cliquables, ouvrant le navigateur par défaut du système.
- **Fiche détail d'un film** : affiche maintenant la jaquette
  (auparavant seules les vignettes liste/grille l'affichaient) ;
  genre, studio, format et pays sont cliquables, et sautent
  directement au groupe correspondant dans le panneau de gauche.
- **La recherche** correspond maintenant aussi aux noms d'acteurs, pas
  seulement au titre.
- L'export HTML statique (`vc_export --html`) a maintenant la même
  bascule **EN / FR** que l'application — il restait auparavant en
  anglais uniquement.
- `vc_add commit`/`set-cover` ont gagné un drapeau `--force`, qui lève
  le refus automatique sur un chemin qui ressemble à la vraie
  collection Movie Collector. Une confirmation `YES` tapée dans le
  terminal reste exigée avant toute écriture réelle.
- Une vérification refuse désormais toute écriture si Movie Collector
  lui-même est actuellement lancé.
- `vc_export` a gagné `--collection`/`--list-collections` pour limiter
  un export CSV/JSON à une seule sous-collection.

## [v1.0.0b1] - 2026-11-14

Première version bêta de VideoNojo, taguée sur GitHub, avec un `.dmg`
téléchargeable (`VideoNojo_0.21.0_aarch64.dmg`, non signé — voir
[`README.fr.md`](README.fr.md) pour contourner le message
« endommagé » du Gatekeeper) dès cette toute première release.

Une application macOS (et des outils en ligne de commande
`vc_export`/`vc_add`) pour consulter, ajouter, modifier et supprimer
des films d'une collection Movie Collector — regroupement par genre/
studio/format/pays, recherche, panneaux redimensionnables, plusieurs
onglets de sous-collection, interface bilingue français/anglais,
mémorisation de l'état de session, autocomplétion du formulaire
d'ajout/modification, export CSV/JSON/HTML autonome, et un outil en
ligne de commande pour amorcer une base `.vnc` indépendante.

L'historique de développement au quotidien (une fonctionnalité par
commit) vit dans les messages de commit du projet (`vX.Y.0 : ...`), à
l'intérieur du dépôt.
