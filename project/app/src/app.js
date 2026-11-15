const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// --- resizable panes -------------------------------------------------
// #splitter-v resizes the groups sidebar's width, #splitter-h resizes
// the list pane's height (the detail pane below it just takes
// whatever's left, via flex: 1 1 auto). Sizes persist in localStorage
// ("videonojo_layout"), separate from "videonojo_state"/"videonojo_lang".
function loadLayout() {
  try {
    const raw = localStorage.getItem("videonojo_layout");
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function saveLayout(layout) {
  try {
    localStorage.setItem("videonojo_layout", JSON.stringify(layout));
  } catch {
    // Unavailable — just skip persistence.
  }
}

function makeDraggable(handle, { onDrag, onEnd }) {
  let dragging = false;
  handle.addEventListener("mousedown", (e) => {
    dragging = true;
    handle.classList.add("dragging");
    e.preventDefault();
  });
  window.addEventListener("mousemove", (e) => {
    if (!dragging) return;
    onDrag(e);
  });
  window.addEventListener("mouseup", () => {
    if (!dragging) return;
    dragging = false;
    handle.classList.remove("dragging");
    onEnd();
  });
}

function setupResizablePanes() {
  const layout = loadLayout();
  const groupsEl = document.getElementById("groups");
  const listPaneEl = document.getElementById("list-pane");

  if (layout.groupsWidth) groupsEl.style.flexBasis = `${layout.groupsWidth}px`;
  if (layout.listHeight) listPaneEl.style.flexBasis = `${layout.listHeight}px`;

  makeDraggable(document.getElementById("splitter-v"), {
    onDrag: (e) => {
      const mainRect = document.getElementById("main").getBoundingClientRect();
      const width = Math.min(Math.max(e.clientX - mainRect.left, 140), mainRect.width - 300);
      groupsEl.style.flexBasis = `${width}px`;
    },
    onEnd: () => {
      saveLayout({ ...loadLayout(), groupsWidth: groupsEl.getBoundingClientRect().width });
    },
  });

  makeDraggable(document.getElementById("splitter-h"), {
    onDrag: (e) => {
      const colRect = document.getElementById("right-col").getBoundingClientRect();
      const height = Math.min(Math.max(e.clientY - colRect.top, 100), colRect.height - 150);
      listPaneEl.style.flexBasis = `${height}px`;
    },
    onEnd: () => {
      saveLayout({ ...loadLayout(), listHeight: listPaneEl.getBoundingClientRect().height });
    },
  });
}

let movies = [];
let currentDetailId = null;
let currentPath = null;
let imagesDir = "";
let groupField = "genres";
let selectedGroup = null;
let selectedLetter = null;
let editingMovieId = null;
let currentCollection = null;
let sortKey = "title";
let sortAsc = true;
let viewMode = "list"; // 'list' | 'grid'
let panel = "library"; // 'library' | 'stats'
let page = 1;
const PAGE_SIZE = 60;

// A stable (non-translated) key to group/tab by — a movie with no
// sub-collection set (ZCOLLECTION is nullable) still needs one to
// live under. Kept separate from the *displayed* label (collectionLabel
// below) so a language switch can't desync currentCollection from the
// just-retranslated label it used to match.
const DEFAULT_COLLECTION_KEY = "__default__";
function collectionKey(m) {
  return m.collection || DEFAULT_COLLECTION_KEY;
}
function collectionLabel(key) {
  return key === DEFAULT_COLLECTION_KEY ? t("collection_default") : key;
}

// --- session-state persistence --------------------------------------
// Remembers the open collection, active tab, grouping, and selection
// across launches — separate from "videonojo_lang" (the language
// toggle), which persists independently of everything else here.
function saveState() {
  try {
    localStorage.setItem(
      "videonojo_state",
      JSON.stringify({
        path: currentPath,
        groupField,
        selectedGroup,
        selectedLetter,
        currentCollection,
        search: document.getElementById("search").value,
        selectedId: currentDetailId,
        sortKey,
        sortAsc,
        viewMode,
        page,
      })
    );
  } catch {
    // Unavailable (e.g. private browsing) — just skip persistence.
  }
}

function loadSavedState() {
  try {
    const raw = localStorage.getItem("videonojo_state");
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

// --- i18n -----------------------------------------------------------
// Every user-visible string goes through t() so toggling the language
// re-renders consistently instead of leaving some parts translated
// and others not. Scoped to the JS frontend only — the native macOS
// menu and any error text returned from Rust commands stay as-is
// regardless of the toggle, same deliberate scope limit BookNojo/
// MusicNojo's own i18n work used.
const I18N = {
  en: {
    nav_open: "Open Collection…",
    nav_add: "Add Movie…",
    nav_stats: "📊 Statistics",
    nav_back_to_collection: "← Back to collection",
    search_placeholder: "Search by title…",
    count_movies: "{n} movie(s)",
    group_all: "All",
    azbar_all: "All",
    collection_default: "My Collection",
    opt_group_genres: "Genre",
    opt_group_studios: "Studio",
    opt_group_formats: "Format",
    opt_group_country: "Country",
    opt_sort_title: "Title (A→Z)",
    opt_sort_year_desc: "Year (newest first)",
    opt_sort_rating_desc: "Rating (highest first)",
    opt_sort_country: "Country (A→Z)",
    opt_sort_date_added: "Date added (newest first)",
    list_view_list: "☰ List",
    list_view_grid: "▦ Grid",
    list_prev: "‹ Prev.",
    list_next: "Next ›",
    list_page_info: "Page {page} of {total}",
    col_title: "Title",
    col_year: "Year",
    col_country: "Country",
    col_genres: "Genre(s)",
    detail_select: "Select a movie.",
    detail_edit: "Edit…",
    detail_delete: "Delete…",
    detail_field_original_title: "Original title",
    detail_field_year: "Year",
    detail_field_country: "Country",
    detail_field_series: "Series",
    detail_field_box_set: "Box set",
    detail_field_tv_series: "TV series",
    detail_field_links: "Links",
    yes: "Yes",
    detail_field_genres: "Genre(s)",
    detail_field_studios: "Studio(s)",
    detail_field_formats: "Format(s)",
    detail_field_regions: "Region(s)",
    detail_field_screen_ratios: "Screen ratio(s)",
    detail_field_subtitles: "Subtitle(s)",
    detail_field_audio_tracks: "Audio track(s)",
    detail_field_hdr: "HDR",
    detail_field_extras: "Extra(s)",
    detail_field_tags: "Tag(s)",
    detail_field_cast: "Cast",
    detail_field_crew: "Crew",
    detail_field_rating: "Rating",
    detail_field_notes: "Notes",
    detail_field_plot: "Plot",
    detail_field_date_added: "Date added",
    delete_confirm: 'Delete "{title}"? This cannot be undone.',
    cover_set_failed: "Setting the cover failed: {err}",
    form_title_add: "Add Movie",
    form_title_edit: "Edit: {title}",
    form_label_title: "Title",
    form_label_year: "Release year",
    form_label_rating: "Rating (0–5)",
    form_label_country: "Country",
    form_label_series: "Series",
    form_label_box_set: "Box set",
    form_label_genres: "Genre(s), comma-separated",
    form_label_studios: "Studio(s), comma-separated",
    form_label_formats: "Format(s), comma-separated",
    form_label_regions: "Region(s), comma-separated",
    form_label_screen_ratios: "Screen ratio(s), comma-separated",
    form_label_subtitles: "Subtitle(s), comma-separated",
    form_label_audio_tracks: "Audio track(s), comma-separated",
    form_label_hdr: "HDR, comma-separated",
    form_label_extras: "Extra(s), comma-separated",
    form_label_tags: "Tag(s), comma-separated",
    form_label_actors: "Actor(s), comma-separated",
    form_label_crew: 'Crew, comma-separated ("Name" or "Name:Role")',
    form_label_plot: "Plot",
    form_label_notes: "Notes",
    form_cancel: "Cancel",
    form_next: "Next…",
    form_save: "Save changes",
    add_preview_confirm: "Confirm and add",
    add_preview_new: "{n} new value(s) will be created:",
    add_preview_none: "No new values — every field already exists in this collection.",
    stats_title: "Statistics — {name}",
    stats_movies: "{n} movie(s)",
    stats_no_cover: "{n} with no cover",
    stats_add_cover_hint: "Click to add a cover",
    stats_no_cast: "{n} with no cast",
    stats_genres: "Genres",
    stats_studios: "Studios",
    stats_formats: "Formats",
    stats_countries: "Countries",
    stats_by_decade: "By release decade",
    stats_by_added_year: "By year added",
    stats_no_data: "No data.",
  },
  fr: {
    nav_open: "Ouvrir une collection…",
    nav_add: "Ajouter un film…",
    nav_stats: "📊 Statistiques",
    nav_back_to_collection: "← Retour à la collection",
    search_placeholder: "Rechercher par titre…",
    count_movies: "{n} film(s)",
    group_all: "Tous",
    azbar_all: "Tout",
    collection_default: "Ma collection",
    opt_group_genres: "Genre",
    opt_group_studios: "Studio",
    opt_group_formats: "Format",
    opt_group_country: "Pays",
    opt_sort_title: "Titre (A→Z)",
    opt_sort_year_desc: "Année (plus récent d'abord)",
    opt_sort_rating_desc: "Note (plus élevée d'abord)",
    opt_sort_country: "Pays (A→Z)",
    opt_sort_date_added: "Date d'ajout (plus récent d'abord)",
    list_view_list: "☰ Liste",
    list_view_grid: "▦ Grille",
    list_prev: "‹ Préc.",
    list_next: "Suiv. ›",
    list_page_info: "Page {page} / {total}",
    col_title: "Titre",
    col_year: "Année",
    col_country: "Pays",
    col_genres: "Genre(s)",
    detail_select: "Sélectionnez un film.",
    detail_edit: "Modifier…",
    detail_delete: "Supprimer…",
    detail_field_original_title: "Titre original",
    detail_field_year: "Année",
    detail_field_country: "Pays",
    detail_field_series: "Série",
    detail_field_box_set: "Coffret",
    detail_field_tv_series: "Série TV",
    detail_field_links: "Liens",
    yes: "Oui",
    detail_field_genres: "Genre(s)",
    detail_field_studios: "Studio(s)",
    detail_field_formats: "Format(s)",
    detail_field_regions: "Région(s)",
    detail_field_screen_ratios: "Format(s) d'écran",
    detail_field_subtitles: "Sous-titre(s)",
    detail_field_audio_tracks: "Piste(s) audio",
    detail_field_hdr: "HDR",
    detail_field_extras: "Bonus",
    detail_field_tags: "Étiquette(s)",
    detail_field_cast: "Acteurs",
    detail_field_crew: "Équipe",
    detail_field_rating: "Note",
    detail_field_notes: "Notes",
    detail_field_plot: "Synopsis",
    detail_field_date_added: "Date d'ajout",
    delete_confirm: 'Supprimer « {title} » ? Cette action est irréversible.',
    cover_set_failed: "Échec de la mise à jour de la jaquette : {err}",
    form_title_add: "Ajouter un film",
    form_title_edit: "Modifier : {title}",
    form_label_title: "Titre",
    form_label_year: "Année de sortie",
    form_label_rating: "Note (0–5)",
    form_label_country: "Pays",
    form_label_series: "Série",
    form_label_box_set: "Coffret",
    form_label_genres: "Genre(s), séparés par des virgules",
    form_label_studios: "Studio(s), séparés par des virgules",
    form_label_formats: "Format(s), séparés par des virgules",
    form_label_regions: "Région(s), séparées par des virgules",
    form_label_screen_ratios: "Format(s) d'écran, séparés par des virgules",
    form_label_subtitles: "Sous-titre(s), séparés par des virgules",
    form_label_audio_tracks: "Piste(s) audio, séparées par des virgules",
    form_label_hdr: "HDR, séparés par des virgules",
    form_label_extras: "Bonus, séparés par des virgules",
    form_label_tags: "Étiquette(s), séparées par des virgules",
    form_label_actors: "Acteur(s), séparés par des virgules",
    form_label_crew: 'Équipe, séparée par des virgules ("Nom" ou "Nom:Rôle")',
    form_label_plot: "Synopsis",
    form_label_notes: "Notes",
    form_cancel: "Annuler",
    form_next: "Suivant…",
    form_save: "Enregistrer",
    add_preview_confirm: "Confirmer et ajouter",
    add_preview_new: "{n} nouvelle(s) valeur(s) seront créées :",
    add_preview_none: "Aucune nouvelle valeur — tous les champs existent déjà dans cette collection.",
    stats_title: "Statistiques — {name}",
    stats_movies: "{n} film(s)",
    stats_no_cover: "{n} sans jaquette",
    stats_add_cover_hint: "Cliquer pour ajouter une jaquette",
    stats_no_cast: "{n} sans acteur",
    stats_genres: "Genres",
    stats_studios: "Studios",
    stats_formats: "Formats",
    stats_countries: "Pays",
    stats_by_decade: "Par décennie de sortie",
    stats_by_added_year: "Par année d'ajout",
    stats_no_data: "Aucune donnée.",
  },
};

let lang = localStorage.getItem("videonojo_lang") || "en";

function t(key, vars) {
  let text = (I18N[lang] && I18N[lang][key]) || I18N.en[key] || key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      text = text.replace(`{${k}}`, v);
    }
  }
  return text;
}

function applyStaticLabels() {
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    el.textContent = t(el.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-placeholder]").forEach((el) => {
    el.placeholder = t(el.dataset.i18nPlaceholder);
  });
  document.getElementById("lang-en").classList.toggle("active", lang === "en");
  document.getElementById("lang-fr").classList.toggle("active", lang === "fr");
}

function setLang(newLang) {
  lang = newLang;
  localStorage.setItem("videonojo_lang", lang);
  applyStaticLabels();
  renderTabs();
  renderGroups();
  render(document.getElementById("search").value);
  if (currentDetailId !== null) showDetail(currentDetailId);
  if (panel === "stats") renderStats();
}

function escapeHtml(s) {
  return String(s ?? "").replace(/[&<>"']/g, (c) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
  })[c]);
}

function field(label, value) {
  return value ? `<dt>${label}</dt><dd>${escapeHtml(value)}</dd>` : "";
}

function roleList(label, names) {
  return names && names.length ? `<dt>${label}</dt><dd>${escapeHtml(names.join(", "))}</dd>` : "";
}

function linksBlock(label, links) {
  if (!links || !links.length) return "";
  const items = links
    .filter((l) => l.url)
    .map(
      (l) =>
        `<li><a href="${escapeHtml(l.url)}" data-link-url="${escapeHtml(l.url)}">${escapeHtml(l.description || l.url)}</a></li>`
    )
    .join("");
  return items ? `<dt>${label}</dt><dd><ul class="link-list">${items}</ul></dd>` : "";
}

function creditList(label, credits) {
  if (!credits || !credits.length) return "";
  const text = credits.map((c) => (c.role ? `${c.name} (${c.role})` : c.name)).join(", ");
  return `<dt>${label}</dt><dd>${escapeHtml(text)}</dd>`;
}

// Read-only for now — most movies have exactly one bare "Disc 1" with
// no episodes, which isn't worth showing; a disc only renders when it
// actually carries something (episodes, or one of its own fields).
function discsBlock(discs) {
  const interesting = (discs || []).filter(
    (d) => (d.episodes && d.episodes.length) || d.format || d.storage_device || d.extra_features || d.storage_slot
  );
  if (!interesting.length) return "";
  return interesting
    .map((d) => {
      const meta = [d.format, d.storage_device, d.extra_features, d.storage_slot]
        .filter(Boolean)
        .map(escapeHtml)
        .join(" — ");
      const episodes = (d.episodes || [])
        .map((e) => {
          const airDate = [e.first_air_year, e.first_air_month, e.first_air_day].filter(Boolean).join("-");
          return `<li>${escapeHtml(e.title || "")}${airDate ? ` — ${escapeHtml(airDate)}` : ""}</li>`;
        })
        .join("");
      return `<div class="disc-heading">${escapeHtml(d.title || "")}${meta ? ` (${meta})` : ""}</div>${
        episodes ? `<ol>${episodes}</ol>` : ""
      }`;
    })
    .join("");
}

// The inverse of fieldsFromAddForm's "Name" / "Name:Role" crew entry
// parsing — used to pre-fill the edit form from a loaded movie's
// already-resolved crew list.
function creditsToInputValue(credits) {
  return (credits || []).map((c) => (c.role ? `${c.name}:${c.role}` : c.name)).join(", ");
}

// Country is a single value, the others are repeatable lists — both
// shapes are normalized to an array so group filtering/counting can
// treat them uniformly.
function groupValues(m, f) {
  return f === "country" ? (m.country ? [m.country] : []) : m[f] || [];
}

// One tab per sub-collection, largest first, ties broken
// alphabetically by the *stable* key (not the translated label, so a
// language switch can't reorder tabs).
function computeCollections() {
  const counts = new Map();
  for (const m of movies) {
    const key = collectionKey(m);
    counts.set(key, (counts.get(key) || 0) + 1);
  }
  return [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
}

function renderTabs() {
  const collections = computeCollections();
  if (currentCollection === null || !collections.some(([key]) => key === currentCollection)) {
    currentCollection = collections.length ? collections[0][0] : null;
  }
  document.getElementById("tabs").innerHTML = collections
    .map(
      ([key, count]) => `
      <div class="tab ${key === currentCollection ? "active" : ""}" data-key="${escapeHtml(key)}">
        ${escapeHtml(collectionLabel(key))} <span class="n">${count}</span>
      </div>`
    )
    .join("");
  document.querySelectorAll("#tabs .tab").forEach((el) =>
    el.addEventListener("click", () => {
      currentCollection = el.dataset.key;
      selectedGroup = null;
      renderTabs();
      renderGroups();
      render(document.getElementById("search").value);
      saveState();
    })
  );
}

function scopedMovies() {
  return movies.filter((m) => collectionKey(m) === currentCollection);
}

// First letter used by the A-Z quick-jump bar — non-letters (numbers,
// symbols, accented-but-not-matched edge cases) all bucket under "#".
function letterOf(value) {
  const ch = (value || "").trim().charAt(0).toUpperCase();
  return /[A-Z]/.test(ch) ? ch : "#";
}

function computeAvailableLetters() {
  const set = new Set();
  for (const m of scopedMovies()) {
    for (const v of groupValues(m, groupField)) {
      if (v) set.add(letterOf(v));
    }
  }
  return [...set].sort();
}

function renderAzBar() {
  const el = document.getElementById("azbar");
  el.innerHTML = "";
  const all = document.createElement("button");
  all.textContent = t("azbar_all");
  all.className = selectedLetter === null ? "active" : "";
  all.addEventListener("click", () => {
    selectedLetter = null;
    renderGroups();
    saveState();
  });
  el.appendChild(all);
  for (const letter of computeAvailableLetters()) {
    const btn = document.createElement("button");
    btn.textContent = letter;
    btn.className = selectedLetter === letter ? "active" : "";
    btn.addEventListener("click", () => {
      selectedLetter = letter;
      renderGroups();
      saveState();
    });
    el.appendChild(btn);
  }
}

// Narrows which group values are listed below (not the movie list
// itself — picking a specific group from the narrowed list is still
// what filters the movies), same two-step behavior as MusicNojo/
// BookNojo's own A-Z bar.
function computeGroups() {
  const counts = new Map();
  for (const m of scopedMovies()) {
    for (const v of groupValues(m, groupField)) {
      if (selectedLetter && letterOf(v) !== selectedLetter) continue;
      counts.set(v, (counts.get(v) || 0) + 1);
    }
  }
  return [...counts.entries()].sort((a, b) => a[0].localeCompare(b[0]));
}

function renderGroups() {
  renderAzBar();
  const groups = computeGroups();
  const allClass = selectedGroup === null ? "active" : "";
  const rows = groups
    .map(
      ([value, count]) => `
      <div class="group-row ${selectedGroup === value ? "active" : ""}" data-value="${escapeHtml(value)}">
        <span class="group-label">${escapeHtml(value)}</span>
        <span class="group-count">${count}</span>
      </div>`
    )
    .join("");
  document.getElementById("group-list").innerHTML = `
    <div class="group-row ${allClass}" data-value="">
      <span class="group-label">${t("group_all")}</span>
      <span class="group-count">${scopedMovies().length}</span>
    </div>
    ${rows}
  `;
  document.querySelectorAll("#group-list .group-row").forEach((el) =>
    el.addEventListener("click", () => {
      selectedGroup = el.dataset.value || null;
      page = 1;
      renderGroups();
      render(document.getElementById("search").value);
      saveState();
    })
  );
}

// Each preset maps to one of the three underlying comparable fields —
// the dropdown's "year"/"rating"/"date added" presets aren't separate
// sort keys, they're just a direction on top of the matching column.
const SORT_PRESETS = {
  title: { key: "title", asc: true },
  year_desc: { key: "release_year", asc: false },
  rating_desc: { key: "rating", asc: false },
  country: { key: "country", asc: true },
  date_added_desc: { key: "date_added", asc: false },
};

function syncSortDropdown() {
  const match = Object.entries(SORT_PRESETS).find(([, p]) => p.key === sortKey && p.asc === sortAsc);
  if (match) document.getElementById("sort").value = match[0];
}

function sortRows(rows) {
  rows.sort((a, b) => {
    const va = Array.isArray(a[sortKey]) ? a[sortKey].join(", ") : a[sortKey];
    const vb = Array.isArray(b[sortKey]) ? b[sortKey].join(", ") : b[sortKey];
    const cmp =
      typeof va === "number" || typeof vb === "number"
        ? (va ?? -Infinity) - (vb ?? -Infinity)
        : String(va ?? "").localeCompare(String(vb ?? ""));
    return sortAsc ? cmp : -cmp;
  });
  return rows;
}

function render(filterText) {
  const filter = (filterText || "").toLowerCase();
  const rows = sortRows(
    scopedMovies()
      .filter((m) => m.title.toLowerCase().includes(filter))
      .filter((m) => selectedGroup === null || groupValues(m, groupField).includes(selectedGroup))
  );

  const totalPages = Math.max(1, Math.ceil(rows.length / PAGE_SIZE));
  page = Math.min(Math.max(1, page), totalPages);
  const start = (page - 1) * PAGE_SIZE;
  const pageRows = rows.slice(start, start + PAGE_SIZE);

  document.getElementById("count").textContent = t("count_movies", { n: rows.length });
  const tbody = document.querySelector("#movies tbody");
  tbody.innerHTML = pageRows
    .map(
      (m) => `
      <tr data-id="${m.id}">
        <td>${escapeHtml(m.title)}</td>
        <td>${escapeHtml(m.release_year)}</td>
        <td>${escapeHtml(m.country)}</td>
        <td>${escapeHtml((m.genres || []).join(", "))}</td>
      </tr>`
    )
    .join("");
  tbody.querySelectorAll("tr").forEach((tr) =>
    tr.addEventListener("click", () => {
      showDetail(Number(tr.dataset.id));
      saveState();
    })
  );

  renderGridView(pageRows);
  renderPager(totalPages);
}

function renderPager(totalPages) {
  document.getElementById("page-info").textContent = t("list_page_info", { page, total: totalPages });
  document.getElementById("prev-page").disabled = page <= 1;
  document.getElementById("next-page").disabled = page >= totalPages;
}

// The WebView's security sandbox blocks raw file:// loads, so cover
// images go through Tauri's asset protocol instead (only serves paths
// explicitly granted via asset_protocol_scope — see src-tauri/lib.rs).
function coverSrc(filename) {
  if (!filename || !imagesDir) return null;
  return window.__TAURI__.core.convertFileSrc(`${imagesDir}/${filename}`);
}

function renderGridView(rows) {
  const grid = document.getElementById("grid-view");
  grid.innerHTML = rows
    .map((m) => {
      const src = coverSrc(m.front_cover);
      const thumb = src
        ? `<img class="thumb" loading="lazy" src="${src}" alt="">`
        : `<div class="thumb"></div>`;
      return `<div class="grid-tile" data-id="${m.id}">
        ${thumb}
        <div class="t">${escapeHtml(m.title)}</div>
        <div class="y">${escapeHtml(m.release_year)}</div>
      </div>`;
    })
    .join("");
  grid.querySelectorAll(".grid-tile").forEach((tile) =>
    tile.addEventListener("click", () => {
      showDetail(Number(tile.dataset.id));
      saveState();
    })
  );
}

function showDetail(id) {
  currentDetailId = id;
  const m = movies.find((x) => x.id === id);
  if (!m) return;
  document.getElementById("detail-empty").hidden = true;
  document.getElementById("detail-content").innerHTML = `
    <h2>${escapeHtml(m.title)}</h2>
    <dl>
      ${field(t("detail_field_original_title"), m.title_original)}
      ${field(t("detail_field_year"), m.release_year)}
      ${field(t("detail_field_country"), m.country)}
      ${field(t("detail_field_series"), m.series)}
      ${field(t("detail_field_box_set"), m.box_set)}
      ${field(t("detail_field_tv_series"), m.is_tv_series ? t("yes") : "")}
      ${roleList(t("detail_field_genres"), m.genres)}
      ${roleList(t("detail_field_studios"), m.studios)}
      ${roleList(t("detail_field_formats"), m.formats)}
      ${roleList(t("detail_field_regions"), m.regions)}
      ${roleList(t("detail_field_screen_ratios"), m.screen_ratios)}
      ${roleList(t("detail_field_subtitles"), m.subtitles)}
      ${roleList(t("detail_field_audio_tracks"), m.audio_tracks)}
      ${roleList(t("detail_field_hdr"), m.hdr)}
      ${roleList(t("detail_field_extras"), m.extras)}
      ${roleList(t("detail_field_tags"), m.tags)}
      ${roleList(t("detail_field_cast"), (m.cast || []).map((c) => c.name))}
      ${creditList(t("detail_field_crew"), m.crew)}
      ${field(t("detail_field_rating"), m.rating != null ? String(m.rating) : "")}
      ${field(t("detail_field_notes"), m.notes)}
      ${field(t("detail_field_plot"), m.plot)}
      ${field(t("detail_field_date_added"), m.date_added)}
      ${linksBlock(t("detail_field_links"), m.links)}
    </dl>
    ${discsBlock(m.discs)}
    <div class="detail-actions">
      <button id="detail-edit">${t("detail_edit")}</button>
      <button id="detail-delete">${t("detail_delete")}</button>
    </div>
  `;
  document.querySelectorAll("#detail-content a[data-link-url]").forEach((el) =>
    el.addEventListener("click", (e) => {
      e.preventDefault();
      window.__TAURI__.opener.openUrl(el.dataset.linkUrl);
    })
  );
  document.getElementById("detail-edit").addEventListener("click", () => openEditForm(m));
  document.getElementById("detail-delete").addEventListener("click", () => deleteMovieWithConfirm(m));
}

function fileBadge(path) {
  const parts = path.split("/").filter(Boolean);
  const name = parts.pop() || path;
  const parent = parts.pop() || "";
  return `${name} — ${parent}`;
}

// Each entry's values come from the currently loaded collection's own
// data — only helps the first value typed in a comma-separated
// multi-value field (a real chip-style input would be needed for
// more, out of scope here, same limit as BookNojo/MusicNojo's own
// first autocomplete pass).
const DATALIST_FIELDS = [
  { datalist: "dl-country", values: (m) => (m.country ? [m.country] : []) },
  { datalist: "dl-series", values: (m) => (m.series ? [m.series] : []) },
  { datalist: "dl-box_set", values: (m) => (m.box_set ? [m.box_set] : []) },
  { datalist: "dl-genres", values: (m) => m.genres || [] },
  { datalist: "dl-studios", values: (m) => m.studios || [] },
  { datalist: "dl-formats", values: (m) => m.formats || [] },
  { datalist: "dl-regions", values: (m) => m.regions || [] },
  { datalist: "dl-screen_ratios", values: (m) => m.screen_ratios || [] },
  { datalist: "dl-subtitles", values: (m) => m.subtitles || [] },
  { datalist: "dl-audio_tracks", values: (m) => m.audio_tracks || [] },
  { datalist: "dl-hdr", values: (m) => m.hdr || [] },
  { datalist: "dl-extras", values: (m) => m.extras || [] },
  { datalist: "dl-tags", values: (m) => m.tags || [] },
  { datalist: "dl-actors", values: (m) => (m.cast || []).map((c) => c.name) },
  { datalist: "dl-crew", values: (m) => (m.crew || []).map((c) => c.name) },
];

function populateDatalists() {
  for (const { datalist, values } of DATALIST_FIELDS) {
    const distinct = new Set();
    for (const m of movies) {
      for (const v of values(m)) distinct.add(v);
    }
    document.getElementById(datalist).innerHTML = [...distinct]
      .sort((a, b) => a.localeCompare(b))
      .map((v) => `<option value="${escapeHtml(v)}"></option>`)
      .join("");
  }
}

// Wrapped in try/catch so a stale saved path (e.g. a collection that
// was moved or deleted since the last launch) reports an error
// instead of throwing out of boot() — an uncaught rejection here
// would skip every event listener boot() wires up afterward, leaving
// every button in the header permanently dead with no way to recover
// short of clearing localStorage by hand. Same shape as MusicNojo's
// own loadCollection.
async function loadCollection(path) {
  try {
    const resp = await invoke("list_movies", { path });
    movies = resp.movies;
    currentPath = resp.path;
    imagesDir = resp.images_dir;
    selectedGroup = null;
    selectedLetter = null;
    page = 1;
    currentCollection = null;
    document.getElementById("file-badge").textContent = fileBadge(resp.path);
    document.getElementById("detail-content").innerHTML = "";
    document.getElementById("detail-empty").hidden = false;
    currentDetailId = null;
    renderTabs();
    renderGroups();
    render(document.getElementById("search").value);
    populateDatalists();
    if (panel === "stats") renderStats();
  } catch (err) {
    document.getElementById("count").textContent = `Error: ${err}`;
  }
}

function listFromCsv(s) {
  return (s || "")
    .split(",")
    .map((x) => x.trim())
    .filter(Boolean);
}

function fieldsFromAddForm(form) {
  const data = new FormData(form);
  return {
    title: data.get("title") || null,
    release_year: data.get("release_year") || null,
    rating: data.get("rating") ? Number(data.get("rating")) : null,
    country: data.get("country") || null,
    series: data.get("series") || null,
    box_set: data.get("box_set") || null,
    genres: listFromCsv(data.get("genres")),
    studios: listFromCsv(data.get("studios")),
    formats: listFromCsv(data.get("formats")),
    regions: listFromCsv(data.get("regions")),
    screen_ratios: listFromCsv(data.get("screen_ratios")),
    subtitles: listFromCsv(data.get("subtitles")),
    audio_tracks: listFromCsv(data.get("audio_tracks")),
    hdr: listFromCsv(data.get("hdr")),
    extras: listFromCsv(data.get("extras")),
    tags: listFromCsv(data.get("tags")),
    actors: listFromCsv(data.get("actors")),
    crew: listFromCsv(data.get("crew")),
    plot: data.get("plot") || null,
    notes: data.get("notes") || null,
    title_original: null,
    upc: null,
    collection: null,
  };
}

let pendingAddFields = null;

function showAddPreview(preview) {
  document.getElementById("add-preview-title").textContent = preview.title;
  document.getElementById("add-preview-summary").textContent =
    preview.new_count > 0 ? t("add_preview_new", { n: preview.new_count }) : t("add_preview_none");
  document.getElementById("add-preview-list").innerHTML = preview.fields
    .map(
      (f) =>
        `<li>${escapeHtml(f.label)}: ${escapeHtml(f.value)}${f.is_new ? ' <span class="new-badge">new</span>' : ""}</li>`
    )
    .join("");
  document.getElementById("add-preview-dialog").showModal();
}

function openEditForm(m) {
  editingMovieId = m.id;
  const addDialog = document.getElementById("add-movie-dialog");
  const addForm = document.getElementById("add-movie-form");
  addForm.reset();
  addForm.elements.title.value = m.title || "";
  addForm.elements.release_year.value = m.release_year || "";
  addForm.elements.rating.value = m.rating ?? "";
  addForm.elements.country.value = m.country || "";
  addForm.elements.series.value = m.series || "";
  addForm.elements.box_set.value = m.box_set || "";
  addForm.elements.genres.value = (m.genres || []).join(", ");
  addForm.elements.studios.value = (m.studios || []).join(", ");
  addForm.elements.formats.value = (m.formats || []).join(", ");
  addForm.elements.regions.value = (m.regions || []).join(", ");
  addForm.elements.screen_ratios.value = (m.screen_ratios || []).join(", ");
  addForm.elements.subtitles.value = (m.subtitles || []).join(", ");
  addForm.elements.audio_tracks.value = (m.audio_tracks || []).join(", ");
  addForm.elements.hdr.value = (m.hdr || []).join(", ");
  addForm.elements.extras.value = (m.extras || []).join(", ");
  addForm.elements.tags.value = (m.tags || []).join(", ");
  addForm.elements.actors.value = (m.cast || []).map((c) => c.name).join(", ");
  addForm.elements.crew.value = creditsToInputValue(m.crew);
  addForm.elements.plot.value = m.plot || "";
  addForm.elements.notes.value = m.notes || "";
  document.getElementById("add-movie-dialog-title").textContent = t("form_title_edit", { title: m.title });
  document.getElementById("add-movie-submit").textContent = t("form_save");
  document.getElementById("add-movie-error").textContent = "";
  addDialog.showModal();
}

async function deleteMovieWithConfirm(m) {
  // tauri-plugin-dialog patches window.confirm to route through the
  // native dialog — the patched version is async (a Promise), never
  // the synchronous browser primitive, so this must be awaited or the
  // condition below is always truthy regardless of what's clicked.
  const confirmed = await window.confirm(t("delete_confirm", { title: m.title }));
  if (!confirmed) return;
  await invoke("delete_movie", { path: currentPath, id: m.id });
  await loadCollection(currentPath);
}

function setViewMode(mode) {
  viewMode = mode;
  document.getElementById("list-pane").classList.toggle("view-grid", mode === "grid");
  document.getElementById("view-list").classList.toggle("active", mode === "list");
  document.getElementById("view-grid").classList.toggle("active", mode === "grid");
  saveState();
}

// --- Statistics -------------------------------------------------------
function computeStats() {
  const scoped = scopedMovies();
  const tally = (getter) => {
    const m = new Map();
    for (const movie of scoped) {
      for (const v of getter(movie)) {
        if (v) m.set(v, (m.get(v) || 0) + 1);
      }
    }
    return [...m.entries()].sort((a, b) => b[1] - a[1]);
  };
  const decades = new Map();
  for (const movie of scoped) {
    const y = parseInt(movie.release_year, 10);
    if (!y) continue;
    const d = Math.floor(y / 10) * 10;
    decades.set(d, (decades.get(d) || 0) + 1);
  }
  const addedYears = new Map();
  for (const movie of scoped) {
    if (!movie.date_added) continue;
    const y = movie.date_added.slice(0, 4);
    addedYears.set(y, (addedYears.get(y) || 0) + 1);
  }
  const noCoverMovies = scoped.filter((m) => !m.front_cover).sort((a, b) => a.title.localeCompare(b.title));
  const noCastMovies = scoped
    .filter((m) => !(m.cast && m.cast.length))
    .sort((a, b) => a.title.localeCompare(b.title));
  return {
    totalMovies: scoped.length,
    noCover: noCoverMovies.length,
    noCast: noCastMovies.length,
    noCoverMovies,
    noCastMovies,
    genres: tally((m) => m.genres || []),
    studios: tally((m) => m.studios || []),
    formats: tally((m) => m.formats || []),
    countries: tally((m) => (m.country ? [m.country] : [])),
    decades: [...decades.entries()].sort((a, b) => a[0] - b[0]).map(([d, c]) => [`${d}s`, c]),
    addedYears: [...addedYears.entries()].sort((a, b) => a[0].localeCompare(b[0])),
  };
}

function barList(title, entries, limit) {
  const top = limit ? entries.slice(0, limit) : entries;
  const max = top.reduce((m, [, c]) => Math.max(m, c), 1);
  const rows = top
    .map(
      ([label, count]) => `
    <div class="stat-row">
      <span class="label">${escapeHtml(label)}</span>
      <span class="bar-wrap"><span class="bar" style="width:${((count / max) * 100).toFixed(1)}%"></span></span>
      <span class="val">${count}</span>
    </div>`
    )
    .join("");
  return `<div class="stat-block"><h3>${escapeHtml(title)}</h3>${rows || `<p class="stat-empty">${escapeHtml(t("stats_no_data"))}</p>`}</div>`;
}

// Which "N with no cover"/"N with no cast" summary items are
// currently expanded — a plain module-level Set rather than per-call
// state, so it survives the full-innerHTML re-render renderStats()
// does on every toggle/language switch/tab change.
const statsExpanded = new Set();

// The "no cover" list's entries double as a shortcut into
// pick_and_set_cover; "no cast" reuses the same renderer but stays
// plain, since there's no equivalent one-click action for a missing
// cast.
function movieListHtml(list, clickableForCover) {
  if (!list.length) return `<p class="stat-empty">${escapeHtml(t("stats_no_data"))}</p>`;
  return `<ul class="stat-detail-list">${list
    .map((m) => {
      const label = escapeHtml(m.title);
      return clickableForCover
        ? `<li class="clickable" data-movie-id="${m.id}" title="${escapeHtml(t("stats_add_cover_hint"))}">${label}</li>`
        : `<li>${label}</li>`;
    })
    .join("")}</ul>`;
}

function statLink(key, labelKey, n) {
  const expanded = statsExpanded.has(key);
  return `<span class="stat-link${expanded ? " expanded" : ""}" data-stat-key="${key}">${t(labelKey, { n: `<strong>${n}</strong>` })}</span>`;
}

function renderStats() {
  const s = computeStats();
  const detailBlock = (key, labelKey, n, list, clickableForCover) => {
    if (!statsExpanded.has(key)) return "";
    return `<div class="stat-detail"><h4>${escapeHtml(t(labelKey, { n }))}</h4>${movieListHtml(list, clickableForCover)}</div>`;
  };
  document.getElementById("stats-pane").innerHTML = `
    <div class="stats-wrap">
      <h2>${escapeHtml(t("stats_title", { name: collectionLabel(currentCollection) }))}</h2>
      <div class="stat-summary">
        <span>${t("stats_movies", { n: `<strong>${s.totalMovies}</strong>` })}</span>
        ${statLink("noCover", "stats_no_cover", s.noCover)}
        ${statLink("noCast", "stats_no_cast", s.noCast)}
      </div>
      ${detailBlock("noCover", "stats_no_cover", s.noCover, s.noCoverMovies, true)}
      ${detailBlock("noCast", "stats_no_cast", s.noCast, s.noCastMovies, false)}
      <div class="stat-grid">
        ${barList(t("stats_genres"), s.genres, 15)}
        ${barList(t("stats_studios"), s.studios, 10)}
        ${barList(t("stats_formats"), s.formats)}
        ${barList(t("stats_countries"), s.countries)}
        ${barList(t("stats_by_decade"), s.decades)}
        ${barList(t("stats_by_added_year"), s.addedYears)}
      </div>
    </div>`;
  document.querySelectorAll(".stat-link").forEach((el) =>
    el.addEventListener("click", () => {
      const key = el.dataset.statKey;
      if (statsExpanded.has(key)) statsExpanded.delete(key);
      else statsExpanded.add(key);
      renderStats();
    })
  );
  document.querySelectorAll(".stat-detail-list li.clickable").forEach((el) =>
    el.addEventListener("click", () => {
      // Fire-and-forget: the picker is async on the Rust side, result
      // arrives later as a "cover-set"/"cover-set-error" event.
      invoke("pick_and_set_cover", { path: currentPath, id: Number(el.dataset.movieId) });
    })
  );
}

function setPanel(next) {
  panel = next;
  document.body.classList.toggle("panel-stats", panel === "stats");
  document.getElementById("stats-btn").classList.toggle("active", panel === "stats");
  document.getElementById("stats-btn").textContent = panel === "stats" ? t("nav_back_to_collection") : t("nav_stats");
  if (panel === "stats") renderStats();
}

async function boot() {
  applyStaticLabels();
  setupResizablePanes();

  try {
    const version = await invoke("app_version");
    document.getElementById("app-version").textContent = `VideoNojo v${version}`;
  } catch {
    // Non-critical — leave the badge empty.
  }

  const saved = loadSavedState();
  await loadCollection(saved?.path ?? null);

  if (saved) {
    if (saved.groupField) {
      groupField = saved.groupField;
      document.getElementById("group-field").value = groupField;
    }
    currentCollection = saved.currentCollection ?? currentCollection;
    selectedGroup = saved.selectedGroup ?? null;
    selectedLetter = saved.selectedLetter ?? null;
    if (saved.sortKey) {
      sortKey = saved.sortKey;
      sortAsc = !!saved.sortAsc;
      syncSortDropdown();
    }
    if (saved.viewMode === "grid") setViewMode("grid");
    page = saved.page || 1;
    if (saved.search) document.getElementById("search").value = saved.search;
    renderTabs();
    renderGroups();
    render(document.getElementById("search").value);
    if (saved.selectedId != null && movies.some((m) => m.id === saved.selectedId)) {
      showDetail(saved.selectedId);
    }
  }

  document.getElementById("search").addEventListener("input", (e) => {
    page = 1;
    render(e.target.value);
    saveState();
  });
  document.getElementById("open-collection").addEventListener("click", () => invoke("pick_collection"));
  document.getElementById("lang-en").addEventListener("click", () => setLang("en"));
  document.getElementById("lang-fr").addEventListener("click", () => setLang("fr"));
  document.getElementById("group-field").addEventListener("change", (e) => {
    groupField = e.target.value;
    selectedGroup = null;
    selectedLetter = null;
    page = 1;
    renderGroups();
    render(document.getElementById("search").value);
    saveState();
  });
  await listen("collection-picked", (event) => {
    loadCollection(event.payload).then(saveState);
  });
  // Reloads the whole collection (simplest way to pick up the new
  // front_cover everywhere it's shown — table/grid thumbnails, detail
  // panel, the stats "no cover" list/count itself) rather than
  // patching just the one movie in local state.
  await listen("cover-set", () => {
    loadCollection(currentPath).then(() => {
      if (panel === "stats") renderStats();
    });
  });
  await listen("cover-set-error", (event) => {
    alert(t("cover_set_failed", { err: event.payload }));
  });

  const sortEl = document.getElementById("sort");
  sortEl.addEventListener("change", () => {
    const preset = SORT_PRESETS[sortEl.value];
    sortKey = preset.key;
    sortAsc = preset.asc;
    page = 1;
    render(document.getElementById("search").value);
    saveState();
  });
  document.querySelectorAll("#movies th").forEach((th) =>
    th.addEventListener("click", () => {
      const k = th.dataset.k;
      sortAsc = sortKey === k ? !sortAsc : true;
      sortKey = k;
      page = 1;
      syncSortDropdown();
      render(document.getElementById("search").value);
      saveState();
    })
  );
  document.getElementById("view-list").addEventListener("click", () => setViewMode("list"));
  document.getElementById("view-grid").addEventListener("click", () => setViewMode("grid"));
  document.getElementById("prev-page").addEventListener("click", () => {
    page--;
    render(document.getElementById("search").value);
    saveState();
  });
  document.getElementById("next-page").addEventListener("click", () => {
    page++;
    render(document.getElementById("search").value);
    saveState();
  });
  document.getElementById("stats-btn").addEventListener("click", () => {
    setPanel(panel === "stats" ? "library" : "stats");
  });

  const addDialog = document.getElementById("add-movie-dialog");
  const addForm = document.getElementById("add-movie-form");
  const previewDialog = document.getElementById("add-preview-dialog");

  document.getElementById("add-movie").addEventListener("click", () => {
    editingMovieId = null;
    addForm.reset();
    document.getElementById("add-movie-dialog-title").textContent = t("form_title_add");
    document.getElementById("add-movie-submit").textContent = t("form_next");
    document.getElementById("add-movie-error").textContent = "";
    addDialog.showModal();
  });
  document.getElementById("add-movie-cancel").addEventListener("click", () => addDialog.close());

  addForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    const fields = fieldsFromAddForm(addForm);
    try {
      if (editingMovieId !== null) {
        // No preview for Edit — same scope as BookNojo/MusicNojo's own
        // Edit, which also writes directly once submitted.
        const replace = {
          replace_genres: true,
          replace_studios: true,
          replace_formats: true,
          replace_regions: true,
          replace_screen_ratios: true,
          replace_subtitles: true,
          replace_audio_tracks: true,
          replace_hdr: true,
          replace_extras: true,
          replace_tags: true,
          replace_actors: true,
          replace_crew: true,
        };
        await invoke("edit_movie", { path: currentPath, id: editingMovieId, fields, replace });
        addDialog.close();
        await loadCollection(currentPath);
      } else {
        pendingAddFields = fields;
        const preview = await invoke("prepare_add_movie", { path: currentPath, fields });
        addDialog.close();
        showAddPreview(preview);
      }
    } catch (err) {
      document.getElementById("add-movie-error").textContent = String(err);
    }
  });

  document.getElementById("add-preview-cancel").addEventListener("click", () => previewDialog.close());
  document.getElementById("add-preview-confirm").addEventListener("click", async () => {
    await invoke("add_movie", { path: currentPath, fields: pendingAddFields });
    previewDialog.close();
    await loadCollection(currentPath);
  });
}

boot();
