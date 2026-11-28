// Actions shared by several views. They update the store; views re-render from it.
import { api } from "./api";
import { toast } from "./components/toast";
import { errorText, t } from "./i18n";
import { navigate } from "./router";
import { state, update, whileBusy, type Section } from "./store";
import type { OpenResult, Overview } from "./types";

/** Runs an action and shows any error as a notice. Returns `undefined` on error. */
export async function attempt<T>(action: () => Promise<T>): Promise<T | undefined> {
  try {
    return await action();
  } catch (error) {
    toast(errorText(error), "error");
    return undefined;
  }
}

export async function refreshProjects(): Promise<void> {
  const projects = await attempt(() => api.listProjects());
  if (projects) update({ projects });
}

/** Shows a website (loads its overview if it is not the selected one). */
export async function showProject(name: string, section: Section): Promise<void> {
  if (state.project?.name !== name) {
    const overview = await attempt(() => api.overview(name));
    if (!overview) {
      navigate({ kind: "start" });
      return;
    }
    update({ project: overview, section, notice: null });
  } else {
    update({ section });
  }
}

/** Replaces the selected website's overview after a change. */
export function setOverview(overview: Overview): void {
  update({ project: overview });
}

function handleOpened(result: OpenResult | null | undefined): void {
  if (!result) return;
  let message = t("reopened");
  if (result.firstUse) message = t("firstUse");
  else if (result.previousVersion) message = t("updated", { previous: result.previousVersion });
  update({ project: result.overview, notice: { message, changed: result.locallyChanged } });
  void refreshProjects();
  navigate({ kind: "site", name: result.overview.name, section: "overview" });
}

export async function pickPackage(): Promise<void> {
  const result = await whileBusy("open", () => attempt(() => api.pickPackage()));
  handleOpened(result);
}

/** Opens packages handed over by double-click, second app start or drag and drop. */
export async function openPending(): Promise<void> {
  for (;;) {
    const result = await attempt(() => api.takePendingOpen());
    if (!result) return;
    handleOpened(result);
  }
}

export async function togglePreview(): Promise<void> {
  const project = state.project;
  if (!project) return;
  if (state.status.preview) {
    await attempt(() => api.stopPreview());
  } else {
    await whileBusy("preview", () => attempt(() => api.startPreview(project.name)));
  }
  const status = await attempt(() => api.serverStatus());
  if (status) update({ status });
}
