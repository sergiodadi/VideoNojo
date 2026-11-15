# Manuel d'utilisation — application VideoNojo

> 🇬🇧 [Read in English](MANUAL.md)

Ce document explique comment utiliser l'application macOS
**VideoNojo** une fois installée. Pour la télécharger, voir
[`README.fr.md`](README.fr.md) ; pour la compiler depuis les sources,
voir [`BUILD.fr.md`](BUILD.fr.md).

## 1. Ouvrir une collection

- **"Open Collection…"** (dans l'en-tête, ou Fichier → Open
  Collection… dans la barre de menus) ouvre le sélecteur de fichier
  natif macOS pour choisir une collection `.mvcp`. Le sélecteur
  n'accepte que les fichiers `.mvcp`.
- Une collection `.mvcp` créée avec une version de Movie Collector
  différente de celle visée par VideoNojo (20.2.2.1) est refusée à ce
  moment-là, avec un message clair plutôt qu'un plantage ou une
  corruption silencieuse.
- Au tout premier lancement, avant d'avoir rien ouvert, VideoNojo
  affiche une collection d'exemple intégrée, entièrement fictive (8
  films inventés, aucune donnée personnelle) — il y a donc toujours
  quelque chose à voir, sans jamais exposer de vraies données par
  défaut.
- Chaque lancement suivant rouvre la même collection, exactement là où
  on l'avait laissée — onglet actif, regroupement, recherche et film
  sélectionné sont tous mémorisés.
- L'en-tête affiche toujours le nom du fichier ouvert, un badge de
  version, et une bascule de langue EN/FR.

## 2. Parcourir la bibliothèque

- **Trois zones** : le panneau de regroupement à gauche, la liste des
  films en haut à droite, la fiche détail en bas à droite. Les
  séparateurs entre les trois zones se redimensionnent à la souris.
- **Regroupement** par Genre (par défaut), Studio, Format ou Pays,
  avec une barre de raccourci A-Z au-dessus de la liste qui la limite
  aux valeurs commençant par une lettre donnée.
- **Recherche** par titre.
- **Tri** par titre, année, note, pays, ou date d'ajout (plus récent
  d'abord), via le menu déroulant au-dessus de la liste ou en cliquant
  sur un en-tête de colonne (recliquer inverse le sens).
- **Vue liste ou grille** : la grille affiche les jaquettes (quand
  elles existent) à la place des lignes du tableau ; les deux sont
  paginées, 60 films par page.
- **Plusieurs sous-collections** dans un même fichier — des onglets en
  haut permettent de basculer entre elles, en limitant la liste et les
  groupes à celle qui est active.
- **Fiche détail d'un film** : titre, titre original, année de sortie,
  pays, genres, studios, formats, distribution (cast), équipe
  technique (avec le rôle, quand il est connu), note, notes de
  l'utilisateur, synopsis, et date d'ajout.
- **Panneau statistiques** (bouton «📊 Statistiques» dans l'en-tête) :
  nombre total de films, combien n'ont pas de jaquette ou pas
  d'acteur, et des graphiques en barres par genre, studio, format,
  pays, décennie de sortie et année d'ajout — le tout limité à
  l'onglet de sous-collection actif.
- Saisir dans les champs pays/genre/studio/format/acteur/équipe
  technique du formulaire d'ajout/modification propose des
  suggestions d'autocomplétion tirées des valeurs déjà présentes dans
  la collection.

## 3. Ajouter, modifier, supprimer un film

- **Ajouter un film** : un formulaire couvrant le titre, le titre
  original, l'année de sortie, la note, le pays, les genres, les
  studios, les formats, les acteurs et l'équipe technique (séparés par
  des virgules ; une entrée d'équipe technique peut être `"Nom"` ou
  `"Nom:Rôle"`), le synopsis et les notes. Avant toute écriture, un
  aperçu liste chaque valeur saisie (pays, genre, studio, format,
  acteur, équipe technique) avec un badge "nouveau" sur celles qui
  n'existent pas encore dans la collection (pour repérer une faute de
  frappe avant qu'elle devienne une entrée dupliquée permanente) —
  c'est la confirmation de cet aperçu qui ajoute réellement le film.
  Le nouveau film rejoint l'onglet de sous-collection actif au moment
  de l'ajout, et reçoit la date et l'heure actuelles comme date
  d'ajout — modifier le film par la suite ne change jamais cette date.
- **Modifier un film** : depuis sa fiche détail, le même formulaire
  s'ouvre pré-rempli avec les valeurs actuelles, et écrit
  immédiatement (pas d'étape d'aperçu).
- **Supprimer un film** : depuis sa fiche détail, après confirmation.
- Après tout ajout, modification ou suppression, la bibliothèque se
  recharge automatiquement : le changement apparaît immédiatement,
  sans action manuelle nécessaire.
- Il n'est pas encore possible, depuis l'application, de définir
  l'image de jaquette d'un film.

## 4. Interface bilingue

- Une bascule **EN / FR** dans l'en-tête change toute l'interface —
  regroupement, liste, fiche détail, formulaire d'ajout/modification —
  entre l'anglais et le français. Le choix est mémorisé d'une session
  à l'autre (tout comme la taille de la mise en page choisie).

## 5. Sécurité des données

- La vraie collection Movie Collector de l'utilisateur n'est jamais
  modifiée par accident : chaque écriture (ajout, modification,
  suppression) est précédée d'une sauvegarde horodatée du fichier, et
  est purement et simplement refusée — sans aucun contournement
  accessible depuis l'application — si le chemin ressemble à la vraie
  collection.
- VideoNojo peut ouvrir la vraie collection en mode **lecture seule**
  pour la parcourir (l'ouverture se fait toujours en
  `SQLITE_OPEN_READ_ONLY` au niveau de la base), mais aucune des
  actions d'écriture ci-dessus n'a d'exception pour elle.

## 6. Ce que l'application ne fait pas encore

- Définir ou changer l'image de jaquette d'un film.
- Changer la sous-collection d'un film depuis l'application — seul
  l'ajout en place un, celui de l'onglet actif. La commande `vc_add
  edit-plan --collection <nom>` permet de déplacer un film existant
  vers une autre sous-collection déjà existante ; voir
  [`CLI.fr.md`](CLI.fr.md).
- Les disques, épisodes, coffrets, séries, liens, étiquettes, région,
  format d'écran, sous-titres, HDR, bonus, ou pistes audio —
  la structure plus riche propre à Movie Collector, au-delà d'une
  fiche film à plat — n'est modélisée nulle part encore dans VideoNojo,
  ni en lecture ni en écriture.
- Recherche en ligne pour pré-remplir une fiche à partir d'un
  code-barres ou d'une correspondance avec une base de données — pas
  commencé.
- Pas encore signée/notarisée avec un compte développeur Apple — sans
  conséquence pour un usage personnel, mais nécessaire avant de
  distribuer l'application à qui que ce soit d'autre.

## 7. Pour aller plus loin

- [`BUILD.fr.md`](BUILD.fr.md) : compiler l'application depuis les
  sources.
- [`CHANGELOG.fr.md`](CHANGELOG.fr.md) : historique des versions
  publiées sur GitHub.

Pour un usage courant, l'application seule suffit — aucune commande de
terminal n'est nécessaire. Pour les outils en ligne de commande
(`vc_export`/`vc_add`), destinés à un usage plus technique (export
CSV/JSON/HTML, scripts), voir [`CLI.fr.md`](CLI.fr.md).
