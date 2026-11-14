const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let movies = [];
let currentDetailId = null;
let currentPath = null;
let groupField = "genres";
let selectedGroup = null;
let editingMovieId = null;

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

function computeGroups() {
  const counts = new Map();
  for (const m of movies) {
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
      <span class="group-label">All</span>
      <span class="group-count">${movies.length}</span>
    </div>
    ${rows}
  `;
  document.querySelectorAll("#group-list .group-row").forEach((el) =>
    el.addEventListener("click", () => {
      selectedGroup = el.dataset.value || null;
      renderGroups();
      render(document.getElementById("search").value);
    })
  );
}

function render(filterText) {
  const filter = (filterText || "").toLowerCase();
  const rows = movies
    .filter((m) => m.title.toLowerCase().includes(filter))
    .filter((m) => selectedGroup === null || groupValues(m, groupField).includes(selectedGroup));
  document.getElementById("count").textContent = `${rows.length} movie(s)`;
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
    tr.addEventListener("click", () => showDetail(Number(tr.dataset.id)))
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
      ${field("Original title", m.title_original)}
      ${field("Year", m.release_year)}
      ${field("Country", m.country)}
      ${roleList("Genre(s)", m.genres)}
      ${roleList("Studio(s)", m.studios)}
      ${roleList("Format(s)", m.formats)}
      ${roleList("Cast", (m.cast || []).map((c) => c.name))}
      ${creditList("Crew", m.crew)}
      ${field("Rating", m.rating != null ? String(m.rating) : "")}
      ${field("Notes", m.notes)}
      ${field("Plot", m.plot)}
      ${field("Date added", m.date_added)}
    </dl>
    <div class="detail-actions">
      <button id="detail-edit">Edit…</button>
      <button id="detail-delete">Delete…</button>
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

async function loadCollection(path) {
  const resp = await invoke("list_movies", { path });
  movies = resp.movies;
  currentPath = resp.path;
  selectedGroup = null;
  document.getElementById("file-badge").textContent = fileBadge(resp.path);
  document.getElementById("detail-content").innerHTML = "";
  document.getElementById("detail-empty").hidden = false;
  currentDetailId = null;
  renderGroups();
  render(document.getElementById("search").value);
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
    preview.new_count > 0
      ? `${preview.new_count} new value(s) will be created:`
      : "No new values — every field already exists in this collection.";
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
  document.getElementById("add-movie-dialog-title").textContent = `Edit: ${m.title}`;
  document.getElementById("add-movie-submit").textContent = "Save changes";
  document.getElementById("add-movie-error").textContent = "";
  addDialog.showModal();
}

async function deleteMovieWithConfirm(m) {
  // tauri-plugin-dialog patches window.confirm to route through the
  // native dialog — the patched version is async (a Promise), never
  // the synchronous browser primitive, so this must be awaited or the
  // condition below is always truthy regardless of what's clicked.
  const confirmed = await window.confirm(`Delete "${m.title}"? This cannot be undone.`);
  if (!confirmed) return;
  await invoke("delete_movie", { path: currentPath, id: m.id });
  await loadCollection(currentPath);
}

async function boot() {
  try {
    const version = await invoke("app_version");
    document.getElementById("app-version").textContent = `VideoNojo v${version}`;
  } catch {
    // Non-critical — leave the badge empty.
  }

  await loadCollection(null);

  document.getElementById("search").addEventListener("input", (e) => render(e.target.value));
  document.getElementById("open-collection").addEventListener("click", () => invoke("pick_collection"));
  document.getElementById("group-field").addEventListener("change", (e) => {
    groupField = e.target.value;
    selectedGroup = null;
    renderGroups();
    render(document.getElementById("search").value);
  });
  await listen("collection-picked", (event) => loadCollection(event.payload));

  const addDialog = document.getElementById("add-movie-dialog");
  const addForm = document.getElementById("add-movie-form");
  const previewDialog = document.getElementById("add-preview-dialog");

  document.getElementById("add-movie").addEventListener("click", () => {
    editingMovieId = null;
    addForm.reset();
    document.getElementById("add-movie-dialog-title").textContent = "Add Movie";
    document.getElementById("add-movie-submit").textContent = "Next…";
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
