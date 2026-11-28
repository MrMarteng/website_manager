// The session log for experts and support requests.
import { attempt } from "../actions";
import { api } from "../api";
import { toast } from "../components/toast";
import { button, h, icon } from "../dom";
import { t } from "../i18n";

export function renderLog(): HTMLElement {
  const output = h("pre", { class: "log", tabindex: 0 });
  const refresh = async () => {
    const lines = await attempt(() => api.readLog());
    if (lines) {
      output.textContent = lines.join("\n");
      output.scrollTop = output.scrollHeight;
    }
  };
  void refresh();
  return h(
    "div",
    { class: "stack" },
    h(
      "header",
      { class: "page-header" },
      h("div", {}, h("h1", {}, icon("terminal"), t("logTitle")), h("p", { class: "muted" }, t("logHint"))),
      h(
        "div",
        { class: "actions" },
        button(t("logRefresh"), () => void refresh(), { variant: "ghost", iconName: "refresh" }),
        button(t("logExport"), () => void attempt(() => api.exportLog()).then((saved) => saved && toast(t("logSaved"))), {
          iconName: "download",
        }),
      ),
    ),
    h("section", { class: "card" }, output),
  );
}
