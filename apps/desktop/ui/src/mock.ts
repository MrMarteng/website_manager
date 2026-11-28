// UI development without Rust: `npm run ui:mock` serves the window with sample data in a browser.
// Only loaded in Vite mode "mock"; never part of a release build.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

import type { FileEntry, Overview } from "./types";

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

mockWindows("main");
mockIPC((command) => {
  switch (command) {
    case "list_projects":
      return [{ name: overview.name, version: overview.current.version }];
    case "project_overview":
    case "build_site":
    case "save_settings":
      return overview;
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
      return new Promise((resolve) =>
        setTimeout(
          () =>
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
          1200,
        ),
      );
    case "read_log":
      return ["open package cafe-windows.tsweb", "package ok: cafe-glueckseelig 2026.11.30 (Known)"];
    default:
      return null;
  }
}, { shouldMockEvents: true });
