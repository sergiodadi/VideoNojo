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
  aux valeurs commençant par une lettre donnée. La liste des groupes
  est triée par ordre alphabétique par défaut — cliquer sur le nombre
  à côté de « Tous » la trie par nombre de films à la place (recliquer
  revient au tri alphabétique).
- **Recherche** par titre ou par nom d'acteur.
- **Tri** par titre, année, note, pays, ou date d'ajout (plus récent
  d'abord), via le menu déroulant au-dessus de la liste ou en cliquant
  sur un en-tête de colonne (recliquer inverse le sens).
- **Vue liste ou grille** : la grille affiche les jaquettes (quand
  elles existent) à la place des lignes du tableau ; les deux sont
  paginées, 60 films par page.
- **Plusieurs sous-collections** dans un même fichier — des onglets en
  haut permettent de basculer entre elles, en limitant la liste et les
  groupes à celle qui est active.
- **Fiche détail d'un film** : jaquette, titre, titre original, année de sortie,
  pays, série, coffret, si c'est une série TV, genres, studios,
  formats, régions, formats d'écran, sous-titres, pistes audio, HDR,
  bonus, étiquettes, distribution (cast), équipe technique (avec le
  rôle, quand il est connu), note, notes de l'utilisateur, synopsis,
  date d'ajout, et les liens (IMDB, bande-annonce, sites de critiques…)
  — cliquer dessus les ouvre dans le navigateur par défaut du système,
  pas dans l'application — plus, pour une série TV, ses disques et leurs
  épisodes (affichés seulement quand un disque porte réellement
  quelque chose ; un disque vide sans épisode ni contenu propre n'est
  pas affiché). Genre, studio, format et pays sont cliquables : cliquer
  dessus saute directement au groupe correspondant dans le panneau de
  gauche, filtré sur cette valeur exacte.
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
  original, l'année de sortie, la note, le pays, la série, le coffret,
  les genres, les studios, les formats, les régions, les formats
  d'écran, les
  sous-titres, les pistes audio, le HDR, les bonus, les étiquettes,
  les acteurs et l'équipe technique (séparés par des virgules ; une
  entrée d'équipe technique peut être `"Nom"` ou `"Nom:Rôle"`), le
  synopsis et les notes. Avant toute écriture, un aperçu liste chaque
  valeur saisie dans l'un de ces champs avec un badge "nouveau" sur celles qui
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
- Une jaquette peut aussi être choisie directement depuis le
  formulaire d'ajout (bouton « Choisir une image… », facultatif) :
  elle est attachée au film une fois l'ajout confirmé. Si l'ajout
  réussit mais que la jaquette ne peut pas être enregistrée, le film
  est quand même ajouté — il suffit de corriger la jaquette ensuite
  depuis la liste « sans jaquette » du panneau Statistiques, ou avec
  `vc_add set-cover`.
- **Définir une jaquette** : depuis le panneau statistiques, dérouler
  « N sans jaquette » puis cliquer sur un film de cette liste ouvre le
  sélecteur de fichier natif macOS pour choisir une image de jaquette
  (avant) — elle est copiée dans le dossier `Images/` de la
  collection. Il n'est pas encore possible de changer la jaquette d'un
  film qui en a déjà une, ni de définir la jaquette arrière, depuis
  l'application (`vc_add set-cover` peut faire les deux ; voir
  [`CLI.fr.md`](CLI.fr.md)).

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
- Chaque écriture est aussi refusée purement et simplement si
  **Movie Collector lui-même est actuellement lancé** — écrire dans le
  fichier pendant que l'application le garde ouvert risque un résultat
  incohérent, ou que l'application écrase le changement à sa prochaine
  sauvegarde. Fermer Movie Collector d'abord.
- VideoNojo peut ouvrir la vraie collection en mode **lecture seule**
  pour la parcourir (l'ouverture se fait toujours en
  `SQLITE_OPEN_READ_ONLY` au niveau de la base), mais aucune des
  actions d'écriture ci-dessus n'a d'exception pour elle.

## 6. Ce que l'application ne fait pas encore

- Changer la jaquette d'un film qui en a déjà une, la remplacer, ou
  définir la jaquette arrière — la section 3 ci-dessus couvre ce qui
  *est* possible depuis le panneau statistiques.
- Changer la sous-collection d'un film depuis l'application — seul
  l'ajout en place un, celui de l'onglet actif. La commande `vc_add
  edit-plan --collection <nom>` permet de déplacer un film existant
  vers une autre sous-collection déjà existante ; voir
  [`CLI.fr.md`](CLI.fr.md).
- Les liens — lecture seule (affichés sur la fiche détail, cliquables,
  s'ouvrent dans le navigateur système) ; aucun moyen pour l'instant
  d'en ajouter, modifier ou supprimer un, ni depuis l'application ni
  depuis `vc_add`. La collection de test n'a que des liens en ligne
  (IMDB, bande-annonce, sites de critiques) pour vérifier — un lien
  vers un fichier local serait aussi lu et affiché de la même façon,
  mais ce cas précis n'est pas vérifié.
- Les disques et épisodes — lecture seule (affichés sur la fiche
  détail d'un film quand un disque porte réellement quelque chose) ;
  aucun moyen pour l'instant d'ajouter, modifier ou supprimer un
  disque ou un épisode, ni depuis l'application ni depuis `vc_add`.
- La série et le coffret — lecture et écriture (une seule valeur par
  film, comme le pays), mais rien encore pour parcourir *par* série
  ou par coffret (pas de regroupement/onglet pour l'un ou l'autre,
  contrairement à genre/studio/format), ni pour voir tous les films
  d'une série ou d'un coffret donné d'un coup. (Région, format
  d'écran, sous-titres, piste audio, HDR, bonus et étiquette — les
  structures many-to-many — sont pleinement modélisées, en lecture et
  en écriture, depuis la v0.25.0.)
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
