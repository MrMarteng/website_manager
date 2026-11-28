import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import { ApiError, toApiError } from "./api";
import { de } from "./i18n/de";
import { en } from "./i18n/en";
import { errorText, setLanguage, t } from "./i18n";
import { parseRoute, routeHash } from "./router";
import type { Overview } from "./types";
import { buildTree } from "./views/edit";
import { nextStep } from "./views/overview";

const RUST_SOURCES = join(__dirname, "../../src-tauri/src");

describe("texts", () => {
  it("English has exactly the German keys", () => {
    expect(Object.keys(en).sort()).toEqual(Object.keys(de).sort());
  });

  it("every error code used in Rust has a text", () => {
    const codes = new Set<string>();
    for (const file of readdirSync(RUST_SOURCES).filter((name) => name.endsWith(".rs"))) {
      const source = readFileSync(join(RUST_SOURCES, file), "utf8");
      for (const match of source.matchAll(/"code:([a-z-]+)/g)) codes.add(match[1] as string);
    }
    expect(codes.size).toBeGreaterThan(5);
    const missing = [...codes].filter((code) => !(`code:${code}` in de));
    expect(missing).toEqual([]);
  });

  it("fills placeholders and translates error codes", () => {
    setLanguage("de");
    expect(t("restored", { version: "1.2.0" })).toContain("1.2.0");
    expect(errorText(new ApiError({ code: "wrong-password", detail: "" }))).toBe("Das Passwort ist falsch.");
    expect(errorText(new ApiError({ code: null, detail: "Festplatte voll" }))).toContain("Festplatte voll");
    setLanguage("en");
    expect(errorText(new ApiError({ code: "newer-on-server", detail: "1.3.0" }))).toContain("1.3.0");
  });
});

describe("api errors", () => {
  it("normalises command errors and plain values", () => {
    expect(toApiError({ code: "no-credentials", detail: "" }).code).toBe("no-credentials");
    expect(toApiError("boom").detail).toBe("boom");
  });
});

describe("router", () => {
  it("round-trips routes, also with special characters", () => {
    const route = { kind: "site", name: "café glück", section: "publish" } as const;
    expect(parseRoute(routeHash(route))).toEqual(route);
    expect(parseRoute("#/log")).toEqual({ kind: "log" });
    expect(parseRoute("#/site/x/unknown")).toEqual({ kind: "site", name: "x", section: "overview" });
    expect(parseRoute("")).toEqual({ kind: "start" });
  });
});

describe("file tree", () => {
  it("groups paths into folders", () => {
    const tree = buildTree([
      { path: "deploy.conf", size: 1 },
      { path: "dist/index.html", size: 2 },
      { path: "dist/css/a.css", size: 3 },
    ]);
    expect(tree.files.map((file) => file.path)).toEqual(["deploy.conf"]);
    const dist = tree.folders.get("dist");
    expect(dist?.files.map((file) => file.path)).toEqual(["dist/index.html"]);
    expect(dist?.folders.get("css")?.path).toBe("dist/css");
  });
});

describe("next step", () => {
  const base: Overview = {
    name: "cafe",
    publisher: "aa:bb",
    current: { id: "1", version: "1.0", created: "2026-11-27T08:00:00Z", changelog: [], current: true },
    versions: [],
    settings: {},
    detected: { protocol: "sftp", host: "h", user: "u", port: 22, root: "/site", mappings: [], hostKeyPinned: true },
    deployError: null,
    storedCredentials: true,
    packageHasCredentials: false,
    hasCompliance: false,
    hasSources: false,
    canBuild: false,
    lastUpload: null,
  };

  it("guides from access to publishing to up to date", () => {
    setLanguage("en");
    expect(nextStep({ ...base, storedCredentials: false, packageHasCredentials: true })).toBe(t("nextTakeAccess"));
    expect(nextStep(base)).toBe(t("nextPublish"));
    const online = {
      version: "1.0",
      created: "2026-11-27T08:00:00Z",
      uploadedAt: "2026-11-28T08:00:00Z",
      target: "server" as const,
      destination: "h",
    };
    expect(nextStep({ ...base, lastUpload: online })).toBe(t("nextUpToDate"));
    expect(nextStep({ ...base, lastUpload: { ...online, target: "folder" } })).toBe(t("nextPublish"));
    expect(nextStep({ ...base, detected: null })).toBe(t("nextNoDeploy"));
  });
});
