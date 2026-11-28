// Left navigation: websites, sections of the selected website, open package, log.
import { pickPackage } from "../actions";
import { h, icon } from "../dom";
import { currentLanguage, setLanguage, t, type Language, type TextKey } from "../i18n";
import type { IconName } from "../icons";
import { routeHash } from "../router";
import { state, update, type Section } from "../store";
import logoUrl from "../../img/logo.svg";

const SECTION_ITEMS: { section: Section; label: TextKey; iconName: IconName }[] = [
  { section: "overview", label: "navOverview", iconName: "dashboard" },
  { section: "publish", label: "navPublish", iconName: "cloudUpload" },
  { section: "versions", label: "navVersions", iconName: "history" },
  { section: "edit", label: "navEdit", iconName: "edit" },
  { section: "settings", label: "navSettings", iconName: "settings" },
];

function link(href: string, label: string, iconName: IconName, active: boolean): HTMLAnchorElement {
  return h(
    "a",
    { href, class: `nav-item${active ? " active" : ""}`, "aria-current": active ? "page" : false },
    icon(iconName),
    h("span", {}, label),
  );
}

export function renderSidebar(): HTMLElement {
  const selected = state.project?.name;
  const websites = state.projects.map((project) =>
    h(
      "li",
      {},
      link(
        routeHash({ kind: "site", name: project.name, section: "overview" }),
        project.name,
        "world",
        project.name === selected && state.section !== "log",
      ),
    ),
  );

  const sections =
    selected && state.section !== "log"
      ? h(
          "div",
          { class: "nav-group" },
          h("p", { class: "nav-heading" }, selected),
          h(
            "ul",
            {},
            SECTION_ITEMS.map((item) =>
              h(
                "li",
                {},
                link(
                  routeHash({ kind: "site", name: selected, section: item.section }),
                  t(item.label),
                  item.iconName,
                  state.section === item.section,
                ),
              ),
            ),
          ),
        )
      : null;

  const languageSelect = h(
    "select",
    {
      class: "language",
      "aria-label": t("language"),
      onchange: (event: Event) => {
        setLanguage((event.target as HTMLSelectElement).value as Language);
        update({});
      },
    },
    h("option", { value: "de", selected: currentLanguage() === "de" }, "Deutsch"),
    h("option", { value: "en", selected: currentLanguage() === "en" }, "English"),
  );

  return h(
    "nav",
    { class: "sidebar", "aria-label": t("appTitle") },
    h("div", { class: "brand" }, h("img", { src: logoUrl, alt: "", class: "logo" }), h("span", {}, t("appTitle"))),
    websites.length
      ? h("div", { class: "nav-group" }, h("p", { class: "nav-heading" }, t("navWebsites")), h("ul", {}, websites))
      : null,
    sections,
    h(
      "div",
      { class: "nav-footer" },
      h(
        "button",
        { type: "button", class: "nav-item", disabled: state.busy.has("open"), onclick: () => void pickPackage() },
        icon("filePlus"),
        h("span", {}, t("navOpenPackage")),
      ),
      link(routeHash({ kind: "log" }), t("navLog"), "terminal", state.section === "log"),
      languageSelect,
    ),
  );
}
