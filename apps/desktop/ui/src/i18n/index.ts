import { ApiError } from "../api";
import { de, type TextKey } from "./de";
import { en } from "./en";

export type Language = "de" | "en";
export type { TextKey };

const tables: Record<Language, Record<TextKey, string>> = { de, en };
let language: Language = "de";

/** Picks the stored or the system language. */
export function initialLanguage(): Language {
  try {
    const stored = localStorage.getItem("language");
    if (stored === "de" || stored === "en") return stored;
  } catch {
    // Storage may be unavailable; fall back to the system language.
  }
  return (navigator.language || "de").toLowerCase().startsWith("de") ? "de" : "en";
}

export function setLanguage(next: Language): void {
  language = next;
  document.documentElement.lang = next;
  try {
    localStorage.setItem("language", next);
  } catch {
    // Not essential.
  }
}

export function currentLanguage(): Language {
  return language;
}

/** Text for `key`, with `{name}` placeholders replaced. */
export function t(key: TextKey, values: Record<string, string | number> = {}): string {
  let text: string = tables[language][key] ?? de[key];
  for (const [name, value] of Object.entries(values)) {
    text = text.split(`{${name}}`).join(String(value));
  }
  return text;
}

/** Whether a translation exists for an error code from Rust. */
export function hasCode(code: string): boolean {
  return `code:${code}` in de;
}

/** Human-readable text for any error. */
export function errorText(error: unknown): string {
  const apiError = error instanceof ApiError ? error : null;
  if (apiError?.code && hasCode(apiError.code)) {
    return t(`code:${apiError.code}` as TextKey, { detail: apiError.detail });
  }
  const detail = apiError ? apiError.detail : String(error);
  return t("error.generic", { detail });
}

export function formatDate(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : date.toLocaleString(language, { dateStyle: "medium", timeStyle: "short" });
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
