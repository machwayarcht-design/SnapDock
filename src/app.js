// SnapDock V1.6 — frontend controller for the single multi-view window.
// The WebView hosts five layers: the Ctrl+~ frosted panel, the top slide-in
// toast, the centered replace-confirm modal, and the Apple-clean shortcuts /
// about sheets. This script wires them to the Rust commands & events and to
// the shared locale files (locales/{lang}.json).

const T = window.__TAURI__;

// Fire-and-forget invoke (most commands don't return anything we await).
function invoke(cmd, args) {
  if (!T) return;
  const fn = (T.core && T.core.invoke) || (T.tauri && T.tauri.invoke) || T.invoke;
  if (fn) fn(cmd, args);
}

// Invoke that returns the resolved value (used for get_settings).
function invokeAsync(cmd, args) {
  return new Promise((resolve) => {
    if (!T) return resolve(undefined);
    const fn = (T.core && T.core.invoke) || (T.tauri && T.tauri.invoke) || T.invoke;
    if (!fn) return resolve(undefined);
    const p = fn(cmd, args);
    if (p && typeof p.then === "function") p.then(resolve).catch(() => resolve(undefined));
    else resolve(undefined);
  });
}

// ── Element handles ───────────────────────────────────────────────
const panel = document.getElementById("panel");
const toastWrap = document.getElementById("toastWrap");
const toastCard = document.getElementById("toastCard");
const toastIcon = document.getElementById("toastIcon");
const toastText = document.getElementById("toastText");
const modal = document.getElementById("modal");
const modalIcon = document.getElementById("modalIcon");
const modalText = document.getElementById("modalText");
const modalActions = document.getElementById("modalActions");
const sheetShortcuts = document.getElementById("sheetShortcuts");
const scTitle = document.getElementById("scTitle");
const scBody = document.getElementById("scBody");
const sheetAbout = document.getElementById("sheetAbout");
const aboutTitle = document.getElementById("aboutTitle");
const aboutBody = document.getElementById("aboutBody");
const tiles = Array.from(document.querySelectorAll(".layout[data-mode]"));

// ── Localization (single source of truth: locales/{lang}.json) ──
let L = null;

const NOTIFY_DEFAULTS = {
  "save-saved": "Layout {n} saved",
  "save-nolayout": "Arrange windows first (1 / 2 / 3 / 4)",
  "save-confirm": "Layout {n} already exists. Replace it?",
  "load-restored": "Workspace {n} restored",
  "load-empty": "Layout {n} is empty",
  "load-nomatch": "No matching windows found",
};

function tr(path, fallback) {
  if (!L) return fallback;
  let cur = L;
  for (const p of path.split(".")) {
    if (cur && typeof cur === "object" && p in cur) cur = cur[p];
    else return fallback;
  }
  return typeof cur === "string" ? cur : fallback;
}

function fmt(tpl, vars) {
  return String(tpl).replace(/\{(\w+)\}/g, (m, k) =>
    vars && vars[k] != null ? String(vars[k]) : m
  );
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c])
  );
}

async function loadLocale() {
  const cfg = await invokeAsync("get_settings", {});
  const lang = cfg && cfg.language ? cfg.language : "en";
  try {
    const res = await fetch("locales/" + lang + ".json", { cache: "no-store" });
    if (res.ok) {
      L = await res.json();
      return;
    }
  } catch (e) {}
  // Fallback to English so the UI is never blank.
  try {
    L = await (await fetch("locales/en.json", { cache: "no-store" })).json();
  } catch (e) {
    L = {};
  }
}

// ── Frosted layout panel (Ctrl+~) ───────────────────────────────
function openPanel() {
  panel.classList.remove("hidden");
  panel.classList.remove("open", "closing");
  void panel.offsetWidth; // force reflow so the entrance replays
  panel.classList.add("open");
}

function closePanel() {
  if (panel.classList.contains("hidden")) return;
  panel.classList.remove("open");
  panel.classList.add("closing");
  setTimeout(() => {
    panel.classList.add("hidden");
    invoke("hide_panel", {});
  }, 150);
}

function arrange(mode) {
  invoke("arrange", { mode });
  closePanel();
}

function tapTile(tile) {
  const mode = Number(tile.dataset.mode);
  tile.classList.add("tap");
  setTimeout(() => {
    tile.classList.remove("tap");
    arrange(mode);
  }, 120);
}

tiles.forEach((tile) => tile.addEventListener("click", () => tapTile(tile)));

// ── Top slide-in toast (restore / save results) ────────────────
let toastTimer = null;

// Hide every other view INSTANTLY so only the requested layer is visible.
// No window-hide side effects — just DOM classes. Fixes the bug where the
// Ctrl+~ panel stayed painted behind the Ctrl+1 toast (and the analogous
// cases for the modal / sheets).
function hideOtherLayers() {
  panel.classList.add("hidden");
  panel.classList.remove("open", "closing");
  modal.classList.add("hidden");
  modal.classList.remove("show");
  modalActions.innerHTML = "";
  sheetShortcuts.classList.add("hidden");
  sheetShortcuts.classList.remove("show");
  sheetAbout.classList.add("hidden");
  sheetAbout.classList.remove("show");
}

function showToast(icon, text) {
  clearTimeout(toastTimer);
  hideOtherLayers();
  toastIcon.textContent = icon;
  toastText.textContent = text;
  toastWrap.classList.remove("hidden");
  toastCard.classList.remove("in");
  void toastCard.offsetWidth;
  toastCard.classList.add("in");
  toastTimer = setTimeout(hideToast, 1600);
}

function hideToast() {
  toastCard.classList.remove("in");
  setTimeout(() => {
    toastWrap.classList.add("hidden");
    invoke("hide_panel", {});
  }, 360);
}

// ── Centered replace-confirm modal ──────────────────────────────
let modalSlot = 0;

function showModal(icon, text, slot) {
  clearTimeout(toastTimer);
  hideOtherLayers();
  modalSlot = slot;
  modalIcon.textContent = icon;
  modalText.textContent = text;
  modalActions.innerHTML = "";

  const yes = document.createElement("button");
  yes.className = "btn-primary";
  yes.textContent = tr("notify.replace", "Replace");
  yes.onclick = () => {
    invoke("confirm_save", { slot: modalSlot });
    const n = modalSlot + 1;
    hideModal();
    showToast("✓", fmt(tr("notify.save-saved", NOTIFY_DEFAULTS["save-saved"]), { n }));
  };

  const no = document.createElement("button");
  no.className = "btn-ghost";
  no.textContent = tr("notify.cancel", "Cancel");
  no.onclick = () => {
    invoke("cancel_save", {});
    hideModal();
  };

  modalActions.append(yes, no);
  modal.classList.remove("hidden");
  void modal.offsetWidth;
  modal.classList.add("show");
}

function hideModal() {
  modal.classList.remove("show");
  modalActions.innerHTML = "";
  setTimeout(() => {
    modal.classList.add("hidden");
    invoke("hide_panel", {});
  }, 180);
}

// ── Sheets (shortcuts / about) ─────────────────────────────────
function closeSheet(el) {
  el.classList.remove("show");
  setTimeout(() => {
    el.classList.add("hidden");
    invoke("hide_panel", {});
  }, 200);
}

document.querySelectorAll(".sheet-close").forEach((btn) => {
  btn.addEventListener("click", () => {
    const id = btn.getAttribute("data-close");
    const el = document.getElementById(id);
    if (el) closeSheet(el);
  });
});

function renderShortcuts() {
  scTitle.textContent = tr("shortcuts.title", "Keyboard Shortcuts");
  scBody.innerHTML = "";
  const groups = (L && L.shortcuts && L.shortcuts.groups) || [];
  groups.forEach((g) => {
    const gEl = document.createElement("div");
    gEl.className = "sc-group";
    const title = document.createElement("div");
    title.className = "sc-group-title";
    title.textContent = g.title || "";
    gEl.appendChild(title);

    (g.items || []).forEach((it) => {
      const row = document.createElement("div");
      row.className = "sc-row";
      const desc = document.createElement("span");
      desc.className = "sc-desc";
      desc.textContent = it.desc || "";
      const key = document.createElement("span");
      key.className = "sc-key";
      key.textContent = it.key || "";
      row.append(desc, key);
      gEl.appendChild(row);
    });
    scBody.appendChild(gEl);
  });
}

function showShortcuts() {
  renderShortcuts();
  hideOtherLayers();
  sheetShortcuts.classList.remove("hidden");
  void sheetShortcuts.offsetWidth;
  sheetShortcuts.classList.add("show");
}

function renderAbout() {
  aboutTitle.textContent = tr("tray.about", "About");
  const a = (L && L.about) || {};

  aboutBody.innerHTML = "";
  const logo = document.createElement("div");
  logo.className = "about-logo";
  const logoImg = document.createElement("img");
  logoImg.src = "icon.png";
  logoImg.alt = "SnapDock";
  logo.appendChild(logoImg);

  const name = document.createElement("div");
  name.className = "about-name";
  name.textContent = a.name || "SnapDock";

  const version = document.createElement("div");
  version.className = "about-version";
  version.textContent = a.version || "Version 1.7";

  const meta = document.createElement("div");
  meta.className = "about-meta";
  meta.innerHTML =
    (a.designer ? escapeHtml(a.designer) + "<br>" : "") +
    (a.builtWith ? escapeHtml(a.builtWith) + "<br>" : "") +
    (a.copyright ? escapeHtml(a.copyright) : "");

  const links = document.createElement("div");
  links.className = "about-links";

  const gh = document.createElement("button");
  gh.className = "about-link";
  gh.innerHTML = (a.github || "GitHub") + ' <small>↗</small>';
  gh.addEventListener("click", () =>
    invoke("open_url", { url: "https://github.com/" })
  );

  const web = document.createElement("button");
  web.className = "about-link";
  web.innerHTML =
    (a.website || "Official Website") + ' <small>(' + (a.reserved || "reserved") + ")</small>";
  web.disabled = true;

  const upd = document.createElement("button");
  upd.className = "about-link";
  upd.innerHTML =
    (a.updates || "Check for Updates") + ' <small>(' + (a.reserved || "reserved") + ")</small>";
  upd.disabled = true;

  links.append(gh, web, upd);
  aboutBody.append(logo, name, version, meta, links);
}

function showAbout() {
  renderAbout();
  hideOtherLayers();
  sheetAbout.classList.remove("hidden");
  void sheetAbout.offsetWidth;
  sheetAbout.classList.add("show");
}

// ── Rust-driven notifications ───────────────────────────────────
function onNotify(p) {
  const kind = p.kind || "";
  const slot = typeof p.slot === "number" ? p.slot : 0;
  const n = slot + 1;
  const iconFor = {
    "save-saved": "✓",
    "save-nolayout": "!",
    "save-confirm": "↺",
    "load-restored": "◧",
    "load-empty": "○",
    "load-nomatch": "?",
  };
  const icon = iconFor[kind] || "";
  const text = fmt(tr("notify." + kind, NOTIFY_DEFAULTS[kind] || kind), { n });
  if (p.modal) showModal(icon, text, slot);
  else showToast(icon, text);
}

// ── Global key handling (only while the panel owns the window) ──
document.addEventListener("keydown", (e) => {
  // Ctrl/Meta/Alt combos are global shortcuts handled by Rust.
  if (e.ctrlKey || e.metaKey || e.altKey) return;

  if (e.key === "Escape") {
    if (!modal.classList.contains("hidden")) return hideModal();
    if (!sheetShortcuts.classList.contains("hidden")) return closeSheet(sheetShortcuts);
    if (!sheetAbout.classList.contains("hidden")) return closeSheet(sheetAbout);
    return closePanel();
  }

  if (["1", "2", "3", "4"].includes(e.key)) {
    if (panel.classList.contains("hidden")) return; // ignore unless panel is open
    const tile = tiles[Number(e.key) - 1];
    if (tile) tapTile(tile);
  }
});

// ── Event listeners from Rust ───────────────────────────────────
if (T && T.event && T.event.listen) {
  T.event.listen("show-panel", () => openPanel());
  T.event.listen("hide-panel", () => closePanel());
  T.event.listen("show-shortcuts", () => showShortcuts());
  T.event.listen("show-about", () => showAbout());
  T.event.listen("notify", (e) => {
    const p = (e && e.payload) || {};
    onNotify(p);
  });
}

// Load the active language once at startup.
loadLocale();
