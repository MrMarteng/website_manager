// Window logic. Only calls the app's own Rust commands; no file or network access here.
"use strict";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = (id) => document.getElementById(id);

let language = "de";
let project = null; // current Overview
let previewTimer = null;
let allowOlderConfirmed = false; // user accepted uploading over a newer server version

// ---------- Texts ----------

function text(key, values = {}) {
  const table = window.WM_TEXTS[language] || window.WM_TEXTS.de;
  let result = table[key] ?? window.WM_TEXTS.de[key] ?? key;
  for (const [name, value] of Object.entries(values)) {
    result = result.replaceAll(`{${name}}`, value);
  }
  return result;
}

function applyLanguage() {
  document.documentElement.lang = language;
  for (const element of document.querySelectorAll("[data-i18n]")) {
    element.textContent = text(element.dataset.i18n);
  }
  for (const element of document.querySelectorAll("[data-i18n-placeholder]")) {
    element.placeholder = text(element.dataset.i18nPlaceholder);
  }
  if (project) renderProject();
}

/** Turns "code:<name> <detail>" errors into a sentence; other errors stay as they are. */
function errorText(error) {
  const message = String(error);
  const match = message.match(/^(code:[a-z-]+)\s*(.*)$/s);
  if (!match) return message;
  return text(match[1], { detail: match[2] });
}

// ---------- Helpers ----------

function show(id, visible) {
  $(id).classList.toggle("hidden", !visible);
}

function showError(error) {
  $("error").textContent = error ? errorText(error) : "";
  show("error", Boolean(error));
}

async function busy(button, action) {
  if (button) button.disabled = true;
  showError(null);
  try {
    return await action();
  } catch (error) {
    showError(error);
    return undefined;
  } finally {
    if (button) button.disabled = false;
  }
}

function element(tag, properties = {}, children = []) {
  const node = document.createElement(tag);
  Object.assign(node, properties);
  node.append(...children);
  return node;
}

function formatDate(value) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString(language);
}

function formatSize(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

// ---------- Views ----------

async function showStart() {
  project = null;
  stopPreviewTimer();
  show("start-view", true);
  show("project-view", false);
  const projects = await invoke("list_projects").catch(() => []);
  $("projects").replaceChildren(
    ...projects.map((item) =>
      element("li", {}, [
        element("button", {
          className: "link",
          textContent: `${item.name} · ${item.version}`,
          onclick: () => busy(null, async () => openProject(await invoke("project_overview", { name: item.name }))),
        }),
      ]),
    ),
  );
  show("projects-card", projects.length > 0);
}

function openProject(overview, notice = null) {
  project = overview;
  show("start-view", false);
  show("project-view", true);
  $("open-notice").textContent = notice?.message ?? "";
  show("open-notice", Boolean(notice?.message));
  $("changed-files").replaceChildren(
    ...(notice?.changed ?? []).map((path) => element("li", { textContent: path })),
  );
  show("changed-notice", (notice?.changed ?? []).length > 0);
  $("upload-progress-card").classList.add("hidden");
  selectTab("overview");
  renderProject();
  startPreviewTimer();
}

function handleOpenResult(result) {
  if (!result) return;
  let message = text("reopened");
  if (result.firstUse) message = text("firstUse");
  else if (result.previousVersion) message = text("updated", { previous: result.previousVersion });
  openProject(result.overview, { message, changed: result.locallyChanged });
}

function selectTab(name) {
  for (const button of document.querySelectorAll("[data-tab]")) {
    const active = button.dataset.tab === name;
    button.setAttribute("aria-selected", String(active));
  }
  for (const tab of document.querySelectorAll(".tab")) {
    tab.classList.toggle("hidden", tab.id !== `tab-${name}`);
  }
  if (name === "files") loadFiles();
}

function renderProject() {
  const p = project;
  $("project-title").textContent = p.name;
  $("project-version").textContent = `${text("version")} ${p.current.version}`;

  // Overview
  $("fact-version").textContent = p.current.version;
  $("fact-created").textContent = formatDate(p.current.created);
  $("fact-publisher").textContent = p.publisher;
  $("changelog").replaceChildren(...p.current.changelog.map((entry) => element("li", { textContent: entry })));
  show("changelog-box", p.current.changelog.length > 0);
  show("compliance-open", p.hasCompliance);
  show("credentials-card", p.packageHasCredentials && !p.storedCredentials);

  // Upload
  const detected = p.detected;
  show("no-deploy", !detected);
  show("upload-form", Boolean(detected));
  if (p.deployError) showError(p.deployError);
  if (detected) {
    $("server-name").textContent = `(${detected.user}@${detected.host}${detected.root})`;
    $("credentials-state").textContent = text(p.storedCredentials ? "credentialsStored" : "credentialsMissing");
    $("unverified-text").textContent = text("unverified", { host: detected.host });
  }
  updateTargetFields();
  show("website-open", Boolean(p.settings.domain));

  // Versions
  $("versions").replaceChildren(
    ...p.versions.map((version) => {
      const label = `${version.version} · ${formatDate(version.created)}`;
      const actions = version.current
        ? [element("span", { className: "badge", textContent: text("current") })]
        : [
            element("button", { textContent: text("restore"), onclick: (event) => restoreVersion(version, event.target) }),
            element("button", { textContent: text("removeVersion"), onclick: (event) => removeVersion(version, event.target) }),
          ];
      return element("li", { className: "row-item" }, [element("span", { textContent: label }), element("span", { className: "row" }, actions)]);
    }),
  );

  // Settings
  const settings = p.settings;
  $("set-protocol").value = settings.protocol ?? detected?.protocol ?? "sftp";
  for (const [id, key, fallback] of [
    ["set-host", "host", detected?.host],
    ["set-user", "user", detected?.user],
    ["set-port", "port", detected?.port],
    ["set-root", "remoteRoot", detected?.root],
    ["set-domain", "domain", ""],
  ]) {
    $(id).value = settings[key] ?? "";
    $(id).placeholder = fallback ?? "";
  }
  $("set-mappings").value = settings.mappings ?? "";
  $("set-mappings").placeholder = (detected?.mappings ?? [])
    .map((m) => `${m.source} -> ${m.target}${optionsText(m)}`)
    .join("\n");
  $("mappings-table").replaceChildren(
    ...(detected?.mappings ?? []).map((m) =>
      element("tr", {}, [
        element("td", { textContent: m.source }),
        element("td", { textContent: "→" }),
        element("td", { textContent: m.target }),
        element("td", { textContent: m.mode }),
        element("td", { textContent: m.protect.join(", ") }),
      ]),
    ),
  );
  $("access-state").textContent = text(p.storedCredentials ? "accessStored" : "accessMissing");
  show("access-forget", p.storedCredentials);
  show("access-export", p.storedCredentials);
}

function optionsText(mapping) {
  const options = [];
  if (mapping.mode !== "sync") options.push(`mode=${mapping.mode}`);
  if (mapping.protect.length) options.push(`protect=${mapping.protect.join(",")}`);
  return options.length ? ` | ${options.join("; ")}` : "";
}

// ---------- Preview ----------

function setPreviewRunning(url) {
  show("preview-start", !url);
  show("preview-stop", Boolean(url));
  $("preview-url").textContent = url || "";
}

function startPreviewTimer() {
  stopPreviewTimer();
  previewTimer = setInterval(async () => setPreviewRunning(await invoke("preview_status")), 3000);
}

function stopPreviewTimer() {
  if (previewTimer) clearInterval(previewTimer);
  previewTimer = null;
}

// ---------- Upload ----------

function targetKind() {
  return document.querySelector("input[name=target]:checked").value;
}

function updateTargetFields() {
  const server = targetKind() === "server";
  show("password-field", server);
  show("credentials-state", server);
  show("folder-field", !server);
  const needsConfirmation =
    server && project?.detected?.protocol === "sftp" && !project.detected.hostKeyPinned;
  show("unverified-field", Boolean(needsConfirmation));
}

function uploadRequest(dryRun, allowOlder) {
  const target =
    targetKind() === "server"
      ? { kind: "server", password: $("password").value || null }
      : { kind: "localFolder", folder: $("local-folder").value };
  return { target, dryRun, allowOlder, acceptUnverifiedHost: $("accept-unverified").checked };
}

function showChanges(result) {
  const key = result.dryRun ? "changeCounts" : "changeCountsDone";
  $("changes-summary").textContent = text(key, { uploaded: result.uploadedCount, deleted: result.deletedCount });
  $("changes-list").replaceChildren(
    ...result.deleted.map((path) => element("li", { className: "deleted", textContent: `− ${path}` })),
    ...result.uploaded.map((path) => element("li", { textContent: `+ ${path}` })),
  );
  show("changes-details", true);
}

async function runUpload(dryRun, button, allowOlder = false) {
  allowOlderConfirmed = allowOlder;
  show("upload-progress-card", true);
  show("changes-details", false);
  $("upload-status").textContent = text("working");
  $("upload-line").textContent = "";
  let result;
  try {
    button.disabled = true;
    showError(null);
    result = await invoke("upload", { name: project.name, request: uploadRequest(dryRun, allowOlder) });
  } catch (error) {
    const message = String(error);
    button.disabled = false;
    if (message.startsWith("code:newer-on-server")) {
      const detail = message.replace("code:newer-on-server", "").trim();
      if (confirm(text("code:newer-confirm", { detail }))) return runUpload(dryRun, button, true);
    }
    $("upload-status").textContent = "";
    showError(error);
    return undefined;
  }
  button.disabled = false;
  let status = text(result.dryRun ? "uploadDryRun" : "uploadDone");
  if (result.skippedSeed > 0) status += ` ${text("seedSkipped")}`;
  $("upload-status").textContent = status;
  $("upload-line").textContent = "";
  showChanges(result);
  return result;
}

async function uploadNow(button) {
  const preview = await runUpload(true, button);
  if (!preview) return;
  const question = text("confirmUpload", { uploaded: preview.uploadedCount, deleted: preview.deletedCount });
  if (confirm(question)) await runUpload(false, button, allowOlderConfirmed);
}

// ---------- Versions ----------

async function restoreVersion(version, button) {
  if (!confirm(text("confirmRestore", { version: version.version }))) return;
  const overview = await busy(button, () => invoke("restore_version", { name: project.name, id: version.id }));
  if (overview) {
    openProject(overview, { message: text("restored", { version: version.version }) });
    selectTab("upload");
  }
}

async function removeVersion(version, button) {
  const overview = await busy(button, () => invoke("remove_version", { name: project.name, id: version.id }));
  if (overview) {
    project = overview;
    renderProject();
  }
}

// ---------- Settings ----------

function emptyToNull(value) {
  const trimmed = value.trim();
  return trimmed === "" ? null : trimmed;
}

async function saveSettings(event) {
  event.preventDefault();
  const port = emptyToNull($("set-port").value);
  const protocol = $("set-protocol").value;
  const settings = {
    protocol: protocol === project.detected?.protocol && !project.settings.protocol ? null : protocol,
    host: emptyToNull($("set-host").value),
    user: emptyToNull($("set-user").value),
    port: port === null ? null : Number(port),
    remoteRoot: emptyToNull($("set-root").value),
    domain: emptyToNull($("set-domain").value),
    mappings: emptyToNull($("set-mappings").value),
  };
  const overview = await busy(event.submitter, () => invoke("save_settings", { name: project.name, settings }));
  if (overview) {
    project = overview;
    renderProject();
    $("open-notice").textContent = text("saved");
    show("open-notice", true);
  }
}

// ---------- Files ----------

let fileList = [];

async function loadFiles() {
  fileList = await invoke("list_files", { name: project.name }).catch(() => []);
  renderFiles();
}

function renderFiles() {
  const filter = $("file-filter").value.toLowerCase();
  const visible = fileList.filter((file) => file.path.toLowerCase().includes(filter)).slice(0, 500);
  $("files").replaceChildren(
    ...visible.map((file) =>
      element("li", { className: "row-item" }, [
        element("button", {
          className: "link",
          textContent: file.path,
          onclick: () => busy(null, () => invoke("open_file", { name: project.name, path: file.path })),
        }),
        element("span", { className: "muted", textContent: formatSize(file.size) }),
      ]),
    ),
  );
}

// ---------- Log ----------

async function refreshLog() {
  $("log").textContent = (await invoke("read_log")).join("\n");
}

// ---------- Start ----------

async function openPending() {
  const result = await busy(null, () => invoke("take_pending_open"));
  handleOpenResult(result);
}

function init() {
  language = (navigator.language || "de").startsWith("de") ? "de" : "en";
  try {
    language = localStorage.getItem("language") || language;
  } catch {
    // Storage may be unavailable; the browser language is used then.
  }
  $("language").value = language;
  applyLanguage();

  $("language").addEventListener("change", (event) => {
    language = event.target.value;
    try {
      localStorage.setItem("language", language);
    } catch {
      // Not essential.
    }
    applyLanguage();
  });
  $("home").addEventListener("click", showStart);
  $("open-button").addEventListener("click", async (event) => {
    handleOpenResult(await busy(event.target, () => invoke("pick_package")));
  });
  for (const button of document.querySelectorAll("[data-tab]")) {
    button.addEventListener("click", () => selectTab(button.dataset.tab));
  }

  $("credentials-import").addEventListener("click", async (event) => {
    const overview = await busy(event.target, () =>
      invoke("import_package_credentials", { name: project.name, password: $("package-password").value }),
    );
    if (overview) {
      $("package-password").value = "";
      project = overview;
      renderProject();
    }
  });
  $("compliance-open").addEventListener("click", (event) =>
    busy(event.target, () => invoke("open_compliance", { name: project.name })),
  );
  $("preview-start").addEventListener("click", async (event) => {
    const url = await busy(event.target, () => invoke("start_preview", { name: project.name }));
    if (url) setPreviewRunning(url);
  });
  $("preview-stop").addEventListener("click", async (event) => {
    await busy(event.target, () => invoke("stop_preview"));
    setPreviewRunning(null);
  });

  for (const radio of document.querySelectorAll("input[name=target]")) {
    radio.addEventListener("change", updateTargetFields);
  }
  $("upload-check").addEventListener("click", (event) => runUpload(true, event.target));
  $("upload-run").addEventListener("click", (event) => uploadNow(event.target));
  $("website-open").addEventListener("click", (event) =>
    busy(event.target, () => invoke("open_website", { name: project.name })),
  );

  $("settings-form").addEventListener("submit", saveSettings);
  $("settings-reset").addEventListener("click", async (event) => {
    const overview = await busy(event.target, () => invoke("save_settings", { name: project.name, settings: {} }));
    if (overview) {
      project = overview;
      renderProject();
    }
  });
  $("access-export").addEventListener("click", (event) =>
    busy(event.target, () =>
      invoke("export_credentials", { name: project.name, password: $("transfer-password").value }),
    ),
  );
  $("access-import").addEventListener("click", async (event) => {
    const overview = await busy(event.target, () =>
      invoke("import_credentials_file", { name: project.name, password: $("transfer-password").value }),
    );
    if (overview) {
      project = overview;
      renderProject();
    }
  });
  $("access-forget").addEventListener("click", async (event) => {
    if (!confirm(text("confirmForget"))) return;
    const overview = await busy(event.target, () => invoke("forget_credentials", { name: project.name }));
    if (overview) {
      project = overview;
      renderProject();
    }
  });

  $("file-filter").addEventListener("input", renderFiles);
  $("reveal").addEventListener("click", (event) =>
    busy(event.target, () => invoke("reveal_project", { name: project.name })),
  );
  $("log-refresh").addEventListener("click", refreshLog);
  $("log-export").addEventListener("click", (event) => busy(event.target, () => invoke("export_log")));
  $("expert").addEventListener("toggle", refreshLog);

  listen("upload-progress", (event) => {
    $("upload-line").textContent = String(event.payload).trim();
  });
  listen("package-open-requested", openPending);

  showStart();
  openPending();
}

init();
