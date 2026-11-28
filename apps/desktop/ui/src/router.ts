// Hash routes: #/ (start), #/log, #/site/<name>/<section>.
import { SECTIONS, type Section } from "./store";

export type Route =
  | { kind: "start" }
  | { kind: "log" }
  | { kind: "site"; name: string; section: Section };

export function parseRoute(hash: string): Route {
  const parts = hash.replace(/^#\/?/, "").split("/").filter(Boolean).map(decodeURIComponent);
  if (parts[0] === "log") return { kind: "log" };
  if (parts[0] === "site" && parts[1]) {
    const section = (SECTIONS as string[]).includes(parts[2] ?? "") ? (parts[2] as Section) : "overview";
    return { kind: "site", name: parts[1], section };
  }
  return { kind: "start" };
}

export function routeHash(route: Route): string {
  switch (route.kind) {
    case "start":
      return "#/";
    case "log":
      return "#/log";
    case "site":
      return `#/site/${encodeURIComponent(route.name)}/${route.section}`;
  }
}

export function navigate(route: Route): void {
  const hash = routeHash(route);
  if (window.location.hash === hash) {
    window.dispatchEvent(new HashChangeEvent("hashchange"));
  } else {
    window.location.hash = hash;
  }
}
