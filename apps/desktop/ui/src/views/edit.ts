// Edit: rebuild, development mode and the files of the version as a tree.
import { attempt, setOverview } from "../actions";
import { api, events } from "../api";
import { toast } from "../components/toast";
import { button, h, icon } from "../dom";
import { formatSize, t } from "../i18n";
import { state, update, whileBusy } from "../store";
import type { FileEntry, Overview } from "../types";
import { resetPublish } from "./publish";

/** Folder node of the file tree. */
export interface TreeNode {
  name: string;
  path: string;
  folders: Map<string, TreeNode>;
  files: FileEntry[];
}

/** Builds a tree from flat paths (`dist/css/a.css` …). */
export function buildTree(entries: FileEntry[]): TreeNode {
  const root: TreeNode = { name: "", path: "", folders: new Map(), files: [] };
  for (const entry of entries) {
    const parts = entry.path.split("/");
    let node = root;
    for (const part of parts.slice(0, -1)) {
      let next = node.folders.get(part);
      if (!next) {
        next = { name: part, path: node.path ? `${node.path}/${part}` : part, folders: new Map(), files: [] };
        node.folders.set(part, next);
      }
      node = next;
    }
    node.files.push(entry);
  }
  return root;
}

const MAX_FILTER_RESULTS = 300;
const files = new Map<string, FileEntry[]>();
let filter = "";
let buildLine = "";

/** Shows the latest build or dev server line; called once at start-up. */
export function listenToBuildLines(): void {
  void events.onBuildLine((line) => {
    buildLine = line.trim();
    const element = document.getElementById("build-line");
    if (element) element.textContent = buildLine;
  });
}

function fileButton(project: Overview, entry: FileEntry, label: string): HTMLElement {
  return h(
    "button",
    { type: "button", class: "tree-file", onclick: () => void attempt(() => api.openFile(project.name, entry.path)) },
    icon("file"),
    h("span", { class: "tree-name" }, label),
    h("span", { class: "muted small" }, formatSize(entry.size)),
  );
}

function folderElement(project: Overview, node: TreeNode): HTMLElement {
  const list = h("ul", { class: "tree" });
  const details = h(
    "details",
    {},
    h("summary", {}, icon("folder"), h("span", { class: "tree-name" }, node.name)),
    list,
  );
  // Children are created on first opening; large folders such as dist/_astro stay cheap.
  details.addEventListener("toggle", () => {
    if (details.open && !list.childElementCount) list.append(...treeItems(project, node));
  });
  return details;
}

function treeItems(project: Overview, node: TreeNode): HTMLElement[] {
  const folders = [...node.folders.values()].sort((a, b) => a.name.localeCompare(b.name));
  const leafs = [...node.files].sort((a, b) => a.path.localeCompare(b.path));
  return [
    ...folders.map((folder) => h("li", {}, folderElement(project, folder))),
    ...leafs.map((entry) => h("li", {}, fileButton(project, entry, entry.path.split("/").pop() ?? entry.path))),
  ];
}

function renderFiles(project: Overview, container: HTMLElement): void {
  const entries = files.get(project.name);
  if (!entries) {
    container.replaceChildren(h("p", { class: "muted" }, t("working")));
    return;
  }
  const needle = filter.trim().toLowerCase();
  if (needle) {
    const matches = entries.filter((entry) => entry.path.toLowerCase().includes(needle)).slice(0, MAX_FILTER_RESULTS);
    container.replaceChildren(
      matches.length
        ? h("ul", { class: "tree flat" }, matches.map((entry) => h("li", {}, fileButton(project, entry, entry.path))))
        : h("p", { class: "muted" }, t("filesEmpty")),
    );
    return;
  }
  container.replaceChildren(h("ul", { class: "tree" }, treeItems(project, buildTree(entries))));
}

async function loadFiles(project: Overview, container: HTMLElement): Promise<void> {
  const entries = await attempt(() => api.listFiles(project.name));
  files.set(project.name, entries ?? []);
  renderFiles(project, container);
}

async function rebuild(project: Overview): Promise<void> {
  buildLine = "";
  const overview = await whileBusy("build", () => attempt(() => api.buildSite(project.name)));
  if (!overview) return;
  files.delete(project.name);
  resetPublish(project.name);
  setOverview(overview);
  toast(t("buildDone"));
}

async function toggleDev(project: Overview): Promise<void> {
  if (state.status.dev) {
    await attempt(() => api.stopDev());
  } else {
    buildLine = "";
    await whileBusy("dev", () => attempt(() => api.startDev(project.name)));
  }
  const status = await attempt(() => api.serverStatus());
  if (status) update({ status });
}

export function renderEdit(project: Overview): HTMLElement {
  const building = state.busy.has("build") || state.busy.has("dev");
  const progress = building
    ? h(
        "div",
        { class: "progress", role: "status" },
        h("div", { class: "progress-bar indeterminate", role: "progressbar", "aria-label": t("working") }),
        h("p", { class: "small muted", id: "build-line" }, buildLine),
      )
    : null;

  const tree = h("div", { class: "tree-container" });
  const search = h("input", {
    type: "search",
    placeholder: t("fileFilter"),
    "aria-label": t("fileFilter"),
    value: filter,
    oninput: (event: Event) => {
      filter = (event.target as HTMLInputElement).value;
      renderFiles(project, tree);
    },
  });
  if (files.has(project.name)) renderFiles(project, tree);
  else void loadFiles(project, tree);

  return h(
    "div",
    { class: "stack" },
    project.canBuild
      ? h(
          "div",
          { class: "grid-2" },
          h(
            "section",
            { class: "card" },
            h("h2", {}, icon("hammer"), t("buildTitle")),
            h("p", { class: "muted" }, t("buildHint")),
            h("div", { class: "row" }, button(t("buildRun"), () => void rebuild(project), { variant: "primary", iconName: "hammer", disabled: building })),
          ),
          h(
            "section",
            { class: "card" },
            h("h2", {}, icon("code"), t("devTitle")),
            h("p", { class: "muted" }, t("devHint")),
            h(
              "div",
              { class: "row" },
              button(state.status.dev ? t("devStop") : t("devStart"), () => void toggleDev(project), {
                iconName: state.status.dev ? "playerStop" : "playerPlay",
                disabled: building && !state.status.dev,
              }),
            ),
            state.status.dev ? h("p", { class: "status ok" }, icon("circleCheck"), t("devRunning", { url: state.status.dev })) : null,
          ),
        )
      : null,
    progress,
    h(
      "section",
      { class: "card" },
      h("div", { class: "card-head" }, h("h2", {}, icon("folder"), t("filesTitle")), search),
      h("p", { class: "muted small" }, t("filesHint")),
      tree,
    ),
  );
}
