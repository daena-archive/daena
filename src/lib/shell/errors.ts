import { nativeErrorMessage } from "../i18n/native-errors.ts";
import { localizeUi } from "../i18n/ui.ts";

/** Use the original message for protocol decisions such as revision conflicts. */
export function rawErrorMessage(cause: unknown): string {
  if (cause && typeof cause === "object" && "message" in cause && typeof cause.message === "string") {
    return cause.message;
  }
  return String(cause);
}

export function friendlyError(cause: unknown) {
  const message = rawErrorMessage(cause);
  if (
    /(?:window\.__TAURI_INTERNALS__.*undefined|Cannot read properties of undefined \(reading 'invoke'\)|undefined is not an object \(evaluating '[^']*\.invoke'\))/.test(
      message,
    )
  ) {
    return localizeUi(
      "The desktop bridge is unavailable. Open this workspace in the Tauri app to use local project storage.",
    );
  }
  return nativeErrorMessage(message);
}
