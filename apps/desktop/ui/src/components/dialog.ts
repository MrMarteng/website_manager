// Modal confirmation dialog based on <dialog>: focus stays inside, Esc cancels.
import { button, h } from "../dom";
import { t } from "../i18n";

export interface ConfirmOptions {
  title: string;
  text: string;
  confirmLabel: string;
  /** Label of the button that closes without confirming (default "Cancel"). */
  cancelLabel?: string;
  danger?: boolean;
}

/** Resolves to true if the user confirms. */
export function confirmDialog(options: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    const titleId = `dialog-title-${Date.now()}`;
    const dialog = h("dialog", { class: "dialog", "aria-labelledby": titleId });
    const finish = (result: boolean) => {
      dialog.close();
      dialog.remove();
      resolve(result);
    };
    const confirm = button(options.confirmLabel, () => finish(true), {
      variant: options.danger ? "danger" : "primary",
    });
    dialog.append(
      h("h2", { id: titleId }, options.title),
      h("p", {}, options.text),
      h("div", { class: "dialog-actions" }, button(options.cancelLabel ?? t("cancel"), () => finish(false), { variant: "ghost" }), confirm),
    );
    // Esc fires "cancel"; treat it like the cancel button.
    dialog.addEventListener("cancel", (event) => {
      event.preventDefault();
      finish(false);
    });
    document.body.append(dialog);
    dialog.showModal();
    confirm.focus();
  });
}
