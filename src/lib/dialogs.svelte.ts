type PendingConfirm = {
  kind: "confirm";
  title: string;
  message: string;
  confirmLabel: string;
  danger: boolean;
  resolve: (value: boolean) => void;
};

type PendingPrompt = {
  kind: "prompt";
  title: string;
  message: string;
  value: string;
  placeholder: string;
  confirmLabel: string;
  resolve: (value: string | null) => void;
};

type PendingDialog = PendingConfirm | PendingPrompt;

type DialogState = {
  open: boolean;
  current: PendingDialog | null;
  queue: PendingDialog[];
};

export const dialogState: DialogState = $state({
  open: false,
  current: null,
  queue: [],
});

type ConfirmOptions = {
  title: string;
  message: string;
  confirmLabel?: string;
  danger?: boolean;
};

type PromptOptions = {
  title: string;
  message: string;
  value?: string;
  placeholder?: string;
  confirmLabel?: string;
};

/**
 * Native-safe confirm. window.confirm is a silent no-op on macOS Tauri/WKWebView,
 * so every caller must go through this service rendered by <DialogHost />.
 */
export function confirmDialog(options: ConfirmOptions): Promise<boolean> {
  return new Promise<boolean>((resolve) => {
    const request: PendingConfirm = {
      kind: "confirm",
      title: options.title,
      message: options.message,
      confirmLabel: options.confirmLabel ?? "Confirm",
      danger: options.danger ?? false,
      resolve,
    };
    enqueue(request);
  });
}

/**
 * Native-safe text prompt. Returns the trimmed value on confirm, or null on
 * cancel. Empty input counts as a confirmed empty string.
 */
export function promptDialog(options: PromptOptions): Promise<string | null> {
  return new Promise<string | null>((resolve) => {
    const request: PendingPrompt = {
      kind: "prompt",
      title: options.title,
      message: options.message,
      value: options.value ?? "",
      placeholder: options.placeholder ?? "",
      confirmLabel: options.confirmLabel ?? "OK",
      resolve,
    };
    enqueue(request);
  });
}

function enqueue(request: PendingDialog) {
  if (dialogState.open) {
    dialogState.queue.push(request);
    return;
  }
  dialogState.open = true;
  dialogState.current = request;
}

export function resolveDialog(value: boolean | string | null) {
  const { current } = dialogState;
  if (current) current.resolve(value as never);
  dialogState.current = null;
  const next = dialogState.queue.shift();
  if (next) {
    dialogState.current = next;
    return;
  }
  dialogState.open = false;
}
