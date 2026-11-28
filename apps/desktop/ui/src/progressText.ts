// Turns progress events into what the progress display shows (pure, tested).
import type { ProgressView } from "./components/progress";
import { formatDuration, formatSize, t, type TextKey } from "./i18n";
import type { BuildProgress, BuildStep, UploadProgress } from "./types";

type StepGroup = "node" | "dependencies" | "build" | "dev-start";

const STEP_LABELS: Record<BuildStep, TextKey> = {
  "node-download": "stepNodeDownload",
  "node-extract": "stepNodeExtract",
  dependencies: "stepDependencies",
  build: "stepBuild",
  "dev-start": "stepDevStart",
};

function group(step: BuildStep): StepGroup {
  return step === "node-download" || step === "node-extract" ? "node" : step;
}

/** Display for a build (`build`) or the start of development mode (`dev`). */
export function buildProgressView(progress: BuildProgress, mode: "build" | "dev"): Partial<ProgressView> {
  const order: StepGroup[] = ["node", "dependencies", mode === "dev" ? "dev-start" : "build"];
  const number = order.indexOf(group(progress.step)) + 1;
  const title = t("stepOf", { number, total: order.length, label: t(STEP_LABELS[progress.step]) });
  if (progress.step === "node-download" && progress.current !== null) {
    const total = progress.total;
    return {
      title,
      fraction: total ? progress.current / total : null,
      detail: total
        ? t("progressBytes", { done: formatSize(progress.current), total: formatSize(total) })
        : formatSize(progress.current),
    };
  }
  return { title, fraction: null, detail: progress.step === "dependencies" ? t("stepDependenciesHint") : "" };
}

/** Display for checking changes (dry run) and uploading. */
export function uploadProgressView(progress: UploadProgress): Partial<ProgressView> {
  const { stats } = progress;
  let title = progress.dryRun ? t("progressChecking") : t("progressUploading");
  if (progress.mappings > 1) title += ` · ${t("progressArea", { number: progress.mapping, total: progress.mappings })}`;
  if (progress.dryRun) {
    return { title, fraction: null, detail: stats.checks ? t("progressChecked", { count: stats.checks }) : "" };
  }
  const parts: string[] = [];
  if (stats.totalBytes > 0) {
    parts.push(t("progressBytes", { done: formatSize(stats.bytes), total: formatSize(stats.totalBytes) }));
  }
  if (stats.totalTransfers > 0) parts.push(t("progressFiles", { done: stats.transfers, total: stats.totalTransfers }));
  if (stats.eta !== null && stats.eta > 0) parts.push(t("progressEta", { time: formatDuration(stats.eta) }));
  return {
    title,
    fraction: stats.totalBytes > 0 ? stats.bytes / stats.totalBytes : null,
    detail: parts.join(" · "),
  };
}
