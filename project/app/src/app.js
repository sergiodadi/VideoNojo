const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let movies = [];
let currentDetailId = null;
let currentPath = null;
let groupField = "genres";
let selectedGroup = null;
let editingMovieId = null;
let currentCollection = null;

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
        currentCollection,
        search: document.getElementById("search").value,
        selectedId: currentDetailId,
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
    search_placeholder: "Search by title…",
    count_movies: "{n} movie(s)",
    group_all: "All",
    collection_default: "My Collection",
    opt_group_genres: "Genre",
    opt_group_studios: "Studio",
    opt_group_formats: "Format",
    opt_group_country: "Country",
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
    detail_field_genres: "Genre(s)",
    detail_field_studios: "Studio(s)",
    detail_field_formats: "Format(s)",
    detail_field_cast: "Cast",
    detail_field_crew: "Crew",
    detail_field_rating: "Rating",
    detail_field_notes: "Notes",
    detail_field_plot: "Plot",
    detail_field_date_added: "Date added",
    delete_confirm: 'Delete "{title}"? This cannot be undone.',
    form_title_add: "Add Movie",
    form_title_edit: "Edit: {title}",
    form_label_title: "Title",
    form_label_year: "Release year",
    form_label_rating: "Rating (0–5)",
    form_label_country: "Country",
    form_label_genres: "Genre(s), comma-separated",
    form_label_studios: "Studio(s), comma-separated",
    form_label_formats: "Format(s), comma-separated",
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
  },
  fr: {
    nav_open: "Ouvrir une collection…",
    nav_add: "Ajouter un film…",
    search_placeholder: "Rechercher par titre…",
    count_movies: "{n} film(s)",
    group_all: "Tous",
    collection_default: "Ma collection",
    opt_group_genres: "Genre",
    opt_group_studios: "Studio",
    opt_group_formats: "Format",
    opt_group_country: "Pays",
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
    detail_field_genres: "Genre(s)",
    detail_field_studios: "Studio(s)",
    detail_field_formats: "Format(s)",
    detail_field_cast: "Acteurs",
    detail_field_crew: "Équipe",
    detail_field_rating: "Note",
    detail_field_notes: "Notes",
    detail_field_plot: "Synopsis",
    detail_field_date_added: "Date d'ajout",
    delete_confirm: 'Supprimer « {title} » ? Cette action est irréversible.',
    form_title_add: "Ajouter un film",
    form_title_edit: "Modifier : {title}",
    form_label_title: "Titre",
    form_label_year: "Année de sortie",
    form_label_rating: "Note (0–5)",
    form_label_country: "Pays",
    form_label_genres: "Genre(s), séparés par des virgules",
    form_label_studios: "Studio(s), séparés par des virgules",
    form_label_formats: "Format(s), séparés par des virgules",
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

function creditList(label, credits) {
  if (!credits || !credits.length) return "";
  const text = credits.map((c) => (c.role ? `${c.name} (${c.role})` : c.name)).join(", ");
  return `<dt>${label}</dt><dd>${escapeHtml(text)}</dd>`;
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

function computeGroups() {
  const counts = new Map();
  for (const m of scopedMovies()) {
    for (const v of groupValues(m, groupField)) {
      counts.set(v, (counts.get(v) || 0) + 1);
    }
  }
  return [...counts.entries()].sort((a, b) => a[0].localeCompare(b[0]));
}

function renderGroups() {
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
      renderGroups();
      render(document.getElementById("search").value);
      saveState();
    })
  );
}

function render(filterText) {
  const filter = (filterText || "").toLowerCase();
  const rows = scopedMovies()
    .filter((m) => m.title.toLowerCase().includes(filter))
    .filter((m) => selectedGroup === null || groupValues(m, groupField).includes(selectedGroup));
  document.getElementById("count").textContent = t("count_movies", { n: rows.length });
  const tbody = document.querySelector("#movies tbody");
  tbody.innerHTML = rows
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
      ${roleList(t("detail_field_genres"), m.genres)}
      ${roleList(t("detail_field_studios"), m.studios)}
      ${roleList(t("detail_field_formats"), m.formats)}
      ${roleList(t("detail_field_cast"), (m.cast || []).map((c) => c.name))}
      ${creditList(t("detail_field_crew"), m.crew)}
      ${field(t("detail_field_rating"), m.rating != null ? String(m.rating) : "")}
      ${field(t("detail_field_notes"), m.notes)}
      ${field(t("detail_field_plot"), m.plot)}
      ${field(t("detail_field_date_added"), m.date_added)}
    </dl>
    <div class="detail-actions">
      <button id="detail-edit">${t("detail_edit")}</button>
      <button id="detail-delete">${t("detail_delete")}</button>
    </div>
  `;
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
  { datalist: "dl-genres", values: (m) => m.genres || [] },
  { datalist: "dl-studios", values: (m) => m.studios || [] },
  { datalist: "dl-formats", values: (m) => m.formats || [] },
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

async function loadCollection(path) {
  const resp = await invoke("list_movies", { path });
  movies = resp.movies;
  currentPath = resp.path;
  selectedGroup = null;
  currentCollection = null;
  document.getElementById("file-badge").textContent = fileBadge(resp.path);
  document.getElementById("detail-content").innerHTML = "";
  document.getElementById("detail-empty").hidden = false;
  currentDetailId = null;
  renderTabs();
  renderGroups();
  render(document.getElementById("search").value);
  populateDatalists();
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
    genres: listFromCsv(data.get("genres")),
    studios: listFromCsv(data.get("studios")),
    formats: listFromCsv(data.get("formats")),
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
  addForm.elements.genres.value = (m.genres || []).join(", ");
  addForm.elements.studios.value = (m.studios || []).join(", ");
  addForm.elements.formats.value = (m.formats || []).join(", ");
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

async function boot() {
  applyStaticLabels();

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
    if (saved.search) document.getElementById("search").value = saved.search;
    renderTabs();
    renderGroups();
    render(document.getElementById("search").value);
    if (saved.selectedId != null && movies.some((m) => m.id === saved.selectedId)) {
      showDetail(saved.selectedId);
    }
  }

  document.getElementById("search").addEventListener("input", (e) => {
    render(e.target.value);
    saveState();
  });
  document.getElementById("open-collection").addEventListener("click", () => invoke("pick_collection"));
  document.getElementById("lang-en").addEventListener("click", () => setLang("en"));
  document.getElementById("lang-fr").addEventListener("click", () => setLang("fr"));
  document.getElementById("group-field").addEventListener("change", (e) => {
    groupField = e.target.value;
    selectedGroup = null;
    renderGroups();
    render(document.getElementById("search").value);
    saveState();
  });
  await listen("collection-picked", (event) => {
    loadCollection(event.payload).then(saveState);
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
