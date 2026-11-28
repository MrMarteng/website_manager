// Entry point: layout, routing, events.
import "./styles.css";

import { openPending, refreshProjects, showProject } from "./actions";
import { api, events } from "./api";
import { h } from "./dom";
import { initialLanguage, setLanguage, t, type TextKey } from "./i18n";
import { navigate, parseRoute } from "./router";
import { state, subscribe, update, type Section } from "./store";
import { renderEdit, listenToBuildLines } from "./views/edit";
import { renderEmpty } from "./views/empty";
import { renderHeader } from "./views/header";
import { renderLog } from "./views/log";
import { renderOverview } from "./views/overview";
import { listenToUploadLines, renderPublish } from "./views/publish";
import { renderSettings } from "./views/settings";
import { renderSidebar } from "./views/sidebar";
import { renderVersions } from "./views/versions";

const SECTION_TITLES: Record<Section, TextKey> = {
  overview: "navOverview",
  publish: "navPublish",
  versions: "navVersions",
  edit: "navEdit",
  settings: "navSettings",
};

const root = document.getElementById("app") as HTMLElement;

function renderMain(): HTMLElement {
  if (state.section === "log") return renderLog();
  const project = state.project;
  if (!project) return renderEmpty();
  const section = state.section;
  const body = {
    overview: renderOverview,
    publish: renderPublish,
    versions: renderVersions,
    edit: renderEdit,
    settings: renderSettings,
  }[section](project);
  return h(
    "div",
    { class: "page" },
    renderHeader(project),
    section === "overview" ? null : h("h2", { class: "section-title" }, t(SECTION_TITLES[section])),
    body,
  );
}

function render(): void {
  // Keep the scroll position of the content when re-rendering the same view.
  const scroll = document.querySelector(".content")?.scrollTop ?? 0;
  root.replaceChildren(renderSidebar(), h("main", { class: "content", id: "content" }, renderMain()));
  const content = document.querySelector(".content");
  if (content) content.scrollTop = scroll;
}

async function applyRoute(): Promise<void> {
  const route = parseRoute(window.location.hash);
  if (route.kind === "log") {
    update({ section: "log" });
  } else if (route.kind === "site") {
    await showProject(route.name, route.section);
  } else if (state.projects[0]) {
    navigate({ kind: "site", name: state.projects[0].name, section: "overview" });
  } else {
    update({ project: null, section: "overview" });
  }
}

function setUpDropHint(): void {
  // The drop itself is handled by Tauri (drag-drop event); this only shows a hint.
  let depth = 0;
  window.addEventListener("dragenter", () => {
    depth += 1;
    document.body.classList.add("dragging");
  });
  window.addEventListener("dragleave", () => {
    depth = Math.max(0, depth - 1);
    if (!depth) document.body.classList.remove("dragging");
  });
  window.addEventListener("drop", () => {
    depth = 0;
    document.body.classList.remove("dragging");
  });
}

async function start(): Promise<void> {
  setLanguage(initialLanguage());
  document.body.dataset.dropHint = t("dropHint");
  subscribe(render);
  render();
  setUpDropHint();
  listenToUploadLines();
  listenToBuildLines();
  void events.onOpenRequested(() => void openPending());
  void events.onServerStatus((status) => update({ status }));
  window.addEventListener("hashchange", () => void applyRoute());
  document.addEventListener("visibilitychange", async () => {
    if (document.visibilityState === "visible") {
      const status = await api.serverStatus().catch(() => null);
      if (status) update({ status });
    }
  });
  await refreshProjects();
  const status = await api.serverStatus().catch(() => null);
  if (status) update({ status });
  await applyRoute();
  await openPending();
}

// `npm run ui:mock`: sample data instead of Rust, for UI work in a plain browser.
const ready = import.meta.env.MODE === "mock" ? import("./mock") : Promise.resolve();
void ready.then(() => start());
