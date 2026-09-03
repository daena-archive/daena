export function friendlyError(cause: unknown) {
  const message = cause instanceof Error ? cause.message : String(cause);
  return message.includes("invoke") || message.includes("undefined")
    ? "The desktop bridge is unavailable. Open this workspace in the Tauri app to use local project storage."
    : message;
}
