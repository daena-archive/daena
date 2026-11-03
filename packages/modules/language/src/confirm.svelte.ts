export interface ConfirmOptions {
  /** Label for the affirmative button. */
  confirmLabel?: string;
  /** Styles the affirmative button as destructive. Defaults to true, the historic look of this modal. */
  danger?: boolean;
}

type PendingConfirm = {
  title: string;
  message: string;
  confirmLabel: string;
  danger: boolean;
  resolve: (value: boolean) => void;
};

type ConfirmState = {
  open: boolean;
  title: string;
  message: string;
  confirmLabel: string;
  danger: boolean;
  resolve: ((value: boolean) => void) | null;
  queue: PendingConfirm[];
};

export const confirmState: ConfirmState = $state({
  open: false,
  title: "",
  message: "",
  confirmLabel: "Confirm",
  danger: true,
  resolve: null,
  queue: [],
});

export function confirm(title: string, message: string, options: ConfirmOptions = {}): Promise<boolean> {
  return new Promise<boolean>((resolve: (value: boolean) => void) => {
    const request: PendingConfirm = {
      title,
      message,
      confirmLabel: options.confirmLabel ?? "Confirm",
      danger: options.danger ?? true,
      resolve,
    };
    if (confirmState.open) {
      confirmState.queue.push(request);
      return;
    }
    activateConfirm(request);
  });
}

function activateConfirm(request: PendingConfirm) {
  confirmState.open = true;
  confirmState.title = request.title;
  confirmState.message = request.message;
  confirmState.confirmLabel = request.confirmLabel;
  confirmState.danger = request.danger;
  confirmState.resolve = request.resolve;
}

export function resolveConfirm(value: boolean) {
  const { resolve } = confirmState;
  confirmState.queue.shift();
  const next = confirmState.queue[0];
  if (next) {
    activateConfirm(next);
  } else {
    confirmState.open = false;
    confirmState.title = "";
    confirmState.message = "";
    confirmState.confirmLabel = "Confirm";
    confirmState.danger = true;
    confirmState.resolve = null;
  }
  resolve?.(value);
}
