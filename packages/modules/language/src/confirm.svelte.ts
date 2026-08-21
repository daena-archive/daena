type PendingConfirm = {
  title: string;
  message: string;
  resolve: (value: boolean) => void;
};

type ConfirmState = {
  open: boolean;
  title: string;
  message: string;
  resolve: ((value: boolean) => void) | null;
  queue: PendingConfirm[];
};

export const confirmState: ConfirmState = $state({
  open: false,
  title: "",
  message: "",
  resolve: null,
  queue: [],
});

export function confirm(title: string, message: string): Promise<boolean> {
  return new Promise<boolean>((resolve: (value: boolean) => void) => {
    const request: PendingConfirm = { title, message, resolve };
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
    confirmState.resolve = null;
  }
  resolve?.(value);
}
