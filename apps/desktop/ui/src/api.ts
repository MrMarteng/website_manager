// The only place that talks to Rust: one typed function per command, plus the events.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  BuildProgress,
  CommandError,
  FileEntry,
  OpenResult,
  Overview,
  ProjectListItem,
  ProjectSettings,
  ServerStatus,
  UploadProgress,
  UploadRequest,
  UploadResult,
} from "./types";

/** Error thrown by every API function; `code` is translated by `errorText`. */
export class ApiError extends Error {
  readonly code: string | null;
  readonly detail: string;

  constructor(error: CommandError) {
    super(error.code ? `${error.code}: ${error.detail}` : error.detail);
    this.code = error.code;
    this.detail = error.detail;
  }
}

/** Normalises whatever a command rejected with. */
export function toApiError(error: unknown): ApiError {
  if (error instanceof ApiError) return error;
  if (error && typeof error === "object" && "detail" in error) {
    const value = error as Partial<CommandError>;
    return new ApiError({ code: value.code ?? null, detail: String(value.detail ?? "") });
  }
  return new ApiError({ code: null, detail: String(error) });
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toApiError(error);
  }
}

export const api = {
  listProjects: () => call<ProjectListItem[]>("list_projects"),
  pickPackage: () => call<OpenResult | null>("pick_package"),
  openPackage: (path: string) => call<OpenResult>("open_package", { path }),
  takePendingOpen: () => call<OpenResult | null>("take_pending_open"),
  overview: (name: string) => call<Overview>("project_overview", { name }),

  importPackageCredentials: (name: string, password: string) =>
    call<Overview>("import_package_credentials", { name, password }),
  forgetCredentials: (name: string) => call<Overview>("forget_credentials", { name }),
  exportCredentials: (name: string, password: string) =>
    call<boolean>("export_credentials", { name, password }),
  importCredentialsFile: (name: string, password: string) =>
    call<Overview | null>("import_credentials_file", { name, password }),
  saveSettings: (name: string, settings: ProjectSettings) =>
    call<Overview>("save_settings", { name, settings }),

  startPreview: (name: string) => call<string>("start_preview", { name }),
  stopPreview: () => call<void>("stop_preview"),
  serverStatus: () => call<ServerStatus>("server_status"),
  upload: (name: string, request: UploadRequest) => call<UploadResult>("upload", { name, request }),
  buildSite: (name: string) => call<Overview>("build_site", { name }),
  startDev: (name: string) => call<string>("start_dev", { name }),
  stopDev: () => call<void>("stop_dev"),

  restoreVersion: (name: string, id: string) => call<Overview>("restore_version", { name, id }),
  removeVersion: (name: string, id: string) => call<Overview>("remove_version", { name, id }),

  listFiles: (name: string) => call<FileEntry[]>("list_files", { name }),
  openFile: (name: string, path: string) => call<void>("open_file", { name, path }),
  revealProject: (name: string) => call<void>("reveal_project", { name }),
  openCompliance: (name: string) => call<void>("open_compliance", { name }),
  openWebsite: (name: string) => call<void>("open_website", { name }),

  readLog: () => call<string[]>("read_log"),
  /** Stops the running rebuild/development mode start or upload; that call then fails with `cancelled`. */
  cancelTask: (task: "build" | "upload") => call<void>("cancel_task", { task }),
  exportLog: () => call<boolean>("export_log"),
};

export const events = {
  onOpenRequested: (handler: () => void): Promise<UnlistenFn> =>
    listen("package-open-requested", () => handler()),
  onUploadLine: (handler: (line: string) => void): Promise<UnlistenFn> =>
    listen<string>("upload-line", (event) => handler(String(event.payload))),
  onUploadProgress: (handler: (progress: UploadProgress) => void): Promise<UnlistenFn> =>
    listen<UploadProgress>("upload-progress", (event) => handler(event.payload)),
  onBuildLine: (handler: (line: string) => void): Promise<UnlistenFn> =>
    listen<string>("build-line", (event) => handler(String(event.payload))),
  onBuildProgress: (handler: (progress: BuildProgress) => void): Promise<UnlistenFn> =>
    listen<BuildProgress>("build-progress", (event) => handler(event.payload)),
  onServerStatus: (handler: (status: ServerStatus) => void): Promise<UnlistenFn> =>
    listen<ServerStatus>("server-status", (event) => handler(event.payload)),
};
