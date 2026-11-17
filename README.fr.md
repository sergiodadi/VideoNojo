# VideoNojo

> 🇬🇧 [Read in English](README.md)

## Pourquoi ce projet

Gestionnaire de collection de films. Movie Collector pour Mac est une
application Intel qui dépend de Rosetta 2. Apple a annoncé la fin
programmée de Rosetta 2 (support complet jusqu'à macOS 27, largement
retiré à partir de macOS 28, ~automne 2027) — Movie Collector a donc
une espérance de vie limitée sur un Mac à jour. VideoNojo importe la
base de données d'une collection [Movie Collector](https://www.collectorz.com/movie)
(Collectorz) dans ses propres outils — lisant et écrivant une copie de
la collection existante, avec export, consultation, ajout,
modification et suppression — pour qu'elle continue de fonctionner, et
de grandir, sans être limitée à ce que l'application de l'éditeur
propose elle-même, ni dépendre de Movie Collector pour l'un ou pour
l'autre, le jour où il ne tournera plus.

VideoNojo est un projet frère de [BookNojo](https://github.com/sergiodadi/BookNojo)
et [MusicNojo](https://github.com/sergiodadi/MusicNojo) (la même
approche et le même problème de fond avec Rosetta 2, appliqués à Book
Collector et Music Collector respectivement).

> Projet en développement actif, disponible publiquement depuis le
> 17/11/2026.

## Ce qu'il fait aujourd'hui

- **Application macOS** (icône, fenêtre, menus natifs) pour consulter,
  ajouter, modifier et supprimer des films d'une collection — avec
  sauvegarde automatique avant toute écriture, et refus pur et simple
  d'écrire sur ce qui ressemble à la vraie collection.
- Regroupement par genre/studio/format/pays ; recherche ; panneaux
  redimensionnables.
- Plusieurs sous-collections dans un même fichier — des onglets
  permettent de basculer entre elles.
- Interface bilingue français/anglais.
- Autocomplétion (`<datalist>`) sur le formulaire d'ajout/modification,
  à partir des valeurs déjà présentes dans la collection.
- Export en ligne de commande vers **CSV**, **JSON**, et une page
  **HTML** autonome ; un outil en ligne de commande pour amorcer une
  base de données indépendante à partir d'une collection existante.

➡️ Guide complet d'utilisation de l'application : [`MANUAL.fr.md`](MANUAL.fr.md).
Des outils en ligne de commande existent aussi (export CSV/JSON/HTML,
scripts) — facultatifs, non nécessaires pour un usage courant : voir
[`CLI.fr.md`](CLI.fr.md).

## Installation

Télécharger le `.dmg` depuis la [dernière release](https://github.com/sergiodadi/VideoNojo/releases/latest),
l'ouvrir, et glisser VideoNojo.app dans Applications.

> ⚠️ **« VideoNojo est endommagé et ne peut pas être ouvert » ?**
> VideoNojo n'est pas signé avec un certificat développeur Apple, donc
> le Gatekeeper de macOS le met en quarantaine et affiche ce message
> trompeur — l'app n'est pas réellement endommagée. Corriger en
> lançant ceci une fois dans le Terminal, puis ouvrir l'app
> normalement :
> ```bash
> xattr -cr /Applications/VideoNojo.app
> ```
> Si ça ne suffit pas, clic droit (ou Ctrl-clic) sur l'app et choisir
> **Ouvrir** plutôt que double-cliquer.

Pour compiler depuis les sources plutôt que d'utiliser le `.dmg`, voir
[`BUILD.fr.md`](BUILD.fr.md).

> Testé uniquement sur **macOS Tahoe (26)** — non vérifié sur d'autres
> versions de macOS.

## Historique des versions

Voir [`CHANGELOG.fr.md`](CHANGELOG.fr.md).

## Licence

[MIT](LICENSE).

---

Ces paquets ont été construits, testés et publiés avec l'aide de Claude, l'assistant IA d'Anthropic.

Movie Collector et Collectorz/CLZ sont des marques déposées de leurs propriétaires respectifs. VideoNojo est un projet indépendant, non affilié à ces sociétés.
