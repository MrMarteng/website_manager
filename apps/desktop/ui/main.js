// Window logic. Only calls the app's own Rust commands; no file or network access here.
"use strict";

const invoke = window.__TAURI__.core.invoke;
const $ = (id) => document.getElementById(id);

let language = "de";
let openedFolder = null;

function text(key, values = {}) {
  const table = window.WM_TEXTS[language] || window.WM_TEXTS.de;
  let result = table[key] ?? key;
  for (const [name, value] of Object.entries(values)) {
    result = result.replace(`{${name}}`, value);
  }
  return result;
}

function applyLanguage() {
  document.documentElement.lang = language;
  for (const element of document.querySelectorAll("[data-i18n]")) {
    element.textContent = text(element.dataset.i18n);
  }
}

function show(id, visible) {
  $(id).classList.toggle("hidden", !visible);
}

function showError(error) {
  $("error").textContent = error ? String(error) : "";
  show("error", Boolean(error));
}

async function busy(button, action) {
  button.disabled = true;
  showError(null);
  try {
    return await action();
  } catch (error) {
    showError(error);
    return undefined;
  } finally {
    button.disabled = false;
  }
}

function showPackage(summary) {
  openedFolder = summary.folder;
  $("package-title").textContent = summary.name;
  $("package-version").textContent = summary.version;
  $("package-created").textContent = new Date(summary.created).toLocaleString(language);
  $("package-publisher").textContent = summary.publisher;
  show("first-use", summary.firstUse);
  const list = $("changelog");
  list.replaceChildren(
    ...summary.changelog.map((entry) => {
      const item = document.createElement("li");
      item.textContent = entry;
      return item;
    }),
  );
  show("changelog-box", summary.changelog.length > 0);
  show("package-card", true);
  show("preview-card", true);
  show("upload-card", summary.hasDeployConf);
}

function setPreviewRunning(url) {
  show("preview-start", !url);
  show("preview-stop", Boolean(url));
  $("preview-url").textContent = url || "";
}

function uploadTarget() {
  const kind = document.querySelector("input[name=target]:checked").value;
  return kind === "server"
    ? { kind: "server", password: $("password").value }
    : { kind: "localFolder", folder: $("local-folder").value };
}

async function upload(dryRun, button) {
  $("upload-result").textContent = text("working");
  const run = (allowOlder) =>
    invoke("upload", { folder: openedFolder, target: uploadTarget(), dryRun, allowOlder });
  let result = await busy(button, () => run(false));
  if (result === undefined && $("error").textContent.includes("newer version") && confirm(text("newerOnServer"))) {
    result = await busy(button, () => run(true));
  }
  if (result === undefined) {
    $("upload-result").textContent = "";
    return;
  }
  let message = text(result.dryRun ? "uploadDryRun" : "uploadDone", { done: result.done, total: result.total });
  if (result.skippedSeed > 0) {
    message += text("uploadSeedSkipped", { skipped: result.skippedSeed });
  }
  $("upload-result").textContent = message;
  await refreshLog();
}

async function refreshLog() {
  const lines = await invoke("read_log");
  $("log").textContent = lines.join("\n");
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

  $("open-button").addEventListener("click", async (event) => {
    const summary = await busy(event.target, () => invoke("pick_package"));
    if (summary) {
      showPackage(summary);
      setPreviewRunning(null);
    }
  });

  $("preview-start").addEventListener("click", async (event) => {
    const url = await busy(event.target, () => invoke("start_preview", { folder: openedFolder }));
    if (url) setPreviewRunning(url);
  });

  $("preview-stop").addEventListener("click", async (event) => {
    await busy(event.target, () => invoke("stop_preview"));
    setPreviewRunning(null);
  });

  for (const radio of document.querySelectorAll("input[name=target]")) {
    radio.addEventListener("change", () => {
      const server = uploadTarget().kind === "server";
      show("password-field", server);
      show("folder-field", !server);
    });
  }

  $("upload-check").addEventListener("click", (event) => upload(true, event.target));
  $("upload-run").addEventListener("click", (event) => upload(false, event.target));
  $("log-refresh").addEventListener("click", refreshLog);
  $("log-export").addEventListener("click", (event) => busy(event.target, () => invoke("export_log")));
  $("expert").addEventListener("toggle", refreshLog);

  setInterval(async () => {
    if (openedFolder) setPreviewRunning(await invoke("preview_status"));
  }, 3000);
}

init();
