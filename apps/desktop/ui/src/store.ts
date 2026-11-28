// One application state; views read it and re-render after `update`.
import type { Overview, ProjectListItem, ServerStatus } from "./types";

export type Section = "overview" | "publish" | "versions" | "edit" | "settings";
export const SECTIONS: Section[] = ["overview", "publish", "versions", "edit", "settings"];

export interface Notice {
  message: string;
  changed: string[];
}

export interface State {
  projects: ProjectListItem[];
  /** Selected website, `null` on the empty start screen or in the log view. */
  project: Overview | null;
  section: Section | "log";
  status: ServerStatus;
  /** Shown once on the overview after opening a package. */
  notice: Notice | null;
  /** Long-running action in progress, e.g. "upload", "build", "dev", "preview". */
  busy: Set<string>;
}

type Listener = (state: State) => void;

const listeners = new Set<Listener>();

export const state: State = {
  projects: [],
  project: null,
  section: "overview",
  status: { preview: null, dev: null },
  notice: null,
  busy: new Set(),
};

export function subscribe(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Changes the state and notifies the views. */
export function update(change: Partial<State>): void {
  Object.assign(state, change);
  for (const listener of listeners) listener(state);
}

/** Runs `action` while `key` is marked busy (buttons for it are disabled meanwhile). */
export async function whileBusy<T>(key: string, action: () => Promise<T>): Promise<T> {
  const busy = new Set(state.busy);
  busy.add(key);
  update({ busy });
  try {
    return await action();
  } finally {
    const done = new Set(state.busy);
    done.delete(key);
    update({ busy: done });
  }
}
