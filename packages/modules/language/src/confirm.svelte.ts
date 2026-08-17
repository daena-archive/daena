type ConfirmState = {
  open: boolean;
  title: string;
  message: string;
  resolve: ((value: boolean) => void) | null;
};

export const confirmState: ConfirmState = $state({
  open: false,
  title: "",
  message: "",
  resolve: null,
});

export function confirm(title: string, message: string): Promise<boolean> {
  return new Promise<boolean>((resolve: (value: boolean) => void) => {
    confirmState.open = true;
    confirmState.title = title;
    confirmState.message = message;
    confirmState.resolve = resolve;
  });
}

export function resolveConfirm(value: boolean) {
  const { resolve } = confirmState;
  confirmState.open = false;
  confirmState.title = "";
  confirmState.message = "";
  confirmState.resolve = null;
  resolve?.(value);
}
