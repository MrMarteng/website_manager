// UI development without Rust: `npm run ui:mock` serves the window with sample data in a browser.
// Only loaded in Vite mode "mock"; never part of a release build.
import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

import type { BuildProgress, FileEntry, Overview, UploadProgress } from "./types";

const overview: Overview = {
  name: "cafe-glueckseelig",
  publisher: "b1d4:c8d5:b213:ea57",
  current: {
    id: "2026.11.30_20261127T200921Z",
    version: "2026.11.30",
    created: "2026-11-27T20:09:21Z",
    changelog: ["Opening hours added", "Menu updated"],
    current: true,
  },
  versions: [
    { id: "2026.11.30_20261127T200921Z", version: "2026.11.30", created: "2026-11-27T20:09:21Z", changelog: [], current: true },
    { id: "2026.11.27_20261127T140021Z", version: "2026.11.27", created: "2026-11-27T14:00:21Z", changelog: [], current: false },
  ],
  settings: { domain: "glueckseelig.teleschirm.com" },
  detected: {
    protocol: "sftp",
    host: "teleschirm.com",
    user: "web1",
    port: 22,
    root: "/glueckseelig.teleschirm.com",
    mappings: [
      { source: "dist/backend/data", target: "/glueckseelig.teleschirm.com/backend/data", mode: "seed", protect: [] },
      { source: "dist", target: "/glueckseelig.teleschirm.com", mode: "sync", protect: ["backend/data/*.json"] },
    ],
    hostKeyPinned: true,
  },
  deployError: null,
  storedCredentials: true,
  packageHasCredentials: true,
  hasCompliance: true,
  hasSources: true,
  canBuild: true,
  lastUpload: {
    version: "2026.11.27",
    created: "2026-11-27T14:00:21Z",
    uploadedAt: "2026-11-27T15:12:00Z",
    target: "server",
    destination: "teleschirm.com",
  },
};

const files: FileEntry[] = [
  { path: "deploy.conf", size: 612 },
  { path: "known_hosts", size: 734 },
  { path: "dist/index.html", size: 113944 },
  { path: "dist/impressum/index.html", size: 44474 },
  { path: "dist/_astro/index.css", size: 20480 },
  { path: "project/package.json", size: 812 },
  { path: "project/src/pages/index.astro", size: 4211 },
  { path: "project/src/content/speisekarte.md", size: 1830 },
];

let preview: string | null = null;

let cancelled = false;
const CANCELLED = { code: "cancelled", detail: "" };

/** Waits, then fails like Rust does if the task was cancelled meanwhile. */
async function wait(ms: number): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, ms));
  if (cancelled) throw CANCELLED;
}

/** Plays upload statistics like rclone sends them, one per 400 ms. */
async function playUpload(dryRun: boolean): Promise<void> {
  cancelled = false;
  const totalBytes = 178_898;
  for (let tick = 0; tick <= 10; tick += 1) {
    const share = tick / 10;
    const progress: UploadProgress = {
      mapping: 2,
      mappings: 2,
      dryRun,
      stats: {
        bytes: dryRun ? 0 : Math.round(totalBytes * share),
        totalBytes: dryRun ? 0 : totalBytes,
        transfers: dryRun ? 0 : Math.round(3 * share),
        totalTransfers: dryRun ? 0 : 3,
        checks: Math.round(40 * share),
        totalChecks: 40,
        eta: dryRun || tick === 10 ? null : 10 - tick,
        speed: 40_000,
        errors: 0,
      },
    };
    await emit("upload-progress", progress);
    await emit("upload-line", `    INFO: dist/file-${tick}.html: Copied (new)`);
    await wait(400);
  }
}

/** Plays a first build: Node.js download, unpacking, npm ci, astro build. */
async function playBuild(lastStep: "build" | "dev-start"): Promise<void> {
  cancelled = false;
  const total = 48_000_000;
  for (let bytes = 0; bytes <= total; bytes += 8_000_000) {
    await emit("build-progress", { step: "node-download", current: bytes, total } satisfies BuildProgress);
    await wait(250);
  }
  for (const step of ["node-extract", "dependencies", lastStep] as const) {
    await emit("build-progress", { step, current: null, total: null } satisfies BuildProgress);
    await emit("build-line", `${step} …`);
    await wait(900);
  }
}

mockWindows("main");
mockIPC((command, args) => {
  switch (command) {
    case "list_projects":
      return [{ name: overview.name, version: overview.current.version }];
    case "project_overview":
    case "save_settings":
      return overview;
    case "build_site":
      return playBuild("build").then(() => overview);
    // Fails on purpose after its steps, to show how a failed step looks.
    case "start_dev":
      return playBuild("dev-start").then(() => Promise.reject({ code: "dev-start-failed", detail: "" }));
    case "server_status":
      return { preview, dev: null };
    case "start_preview":
      preview = "http://127.0.0.1:52417/";
      return preview;
    case "stop_preview":
      preview = null;
      return null;
    case "list_files":
      return files;
    case "upload":
      return playUpload(Boolean((args as { request?: { dryRun?: boolean } }).request?.dryRun)).then(
        () =>
          new Promise((resolve) =>
            resolve({
              dryRun: true,
              done: 4,
              total: 4,
              skippedSeed: 1,
              uploadedCount: 3,
              deletedCount: 1,
              uploaded: ["/site/index.html", "/site/impressum/index.html", "/site/_astro/index.css"],
              deleted: ["/site/alt.html"],
              website: overview.settings.domain ?? null,
            }),
          ),
      );
    case "cancel_task":
      cancelled = true;
      return null;
    case "read_log":
      return ["open package cafe-windows.tsweb", "package ok: cafe-glueckseelig 2026.11.30 (Known)"];
    default:
      return null;
  }
}, { shouldMockEvents: true });
