// Start screen without a selected website.
import { pickPackage } from "../actions";
import { button, h, icon } from "../dom";
import { t } from "../i18n";
import { state } from "../store";

export function renderEmpty(): HTMLElement {
  return h(
    "section",
    { class: "empty" },
    h("div", { class: "empty-icon" }, icon("filePlus")),
    h("h1", {}, t("emptyTitle")),
    h("p", { class: "muted" }, t("emptyText")),
    button(t("emptyAction"), () => void pickPackage(), {
      variant: "primary",
      iconName: "filePlus",
      disabled: state.busy.has("open"),
    }),
  );
}
