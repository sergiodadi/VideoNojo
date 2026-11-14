const { invoke } = window.__TAURI__.core;

let movies = [];
let currentDetailId = null;

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

function render(filterText) {
  const filter = (filterText || "").toLowerCase();
  const rows = movies.filter((m) => m.title.toLowerCase().includes(filter));
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
  `;
}

async function boot() {
  try {
    const version = await invoke("app_version");
    document.getElementById("app-version").textContent = `VideoNojo v${version}`;
  } catch {
    // Non-critical — leave the badge empty.
  }

  const resp = await invoke("list_movies", { path: null });
  movies = resp.movies;
  render("");

  document.getElementById("search").addEventListener("input", (e) => render(e.target.value));
}

boot();
