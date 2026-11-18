import { invoke } from "@tauri-apps/api/core";

import type { AppError } from "./generated";

/** True when `value` has the shape of an `AppError` from the backend. */
export function isAppError(value: unknown): value is AppError {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Record<string, unknown>;
  return typeof candidate.code === "string" && typeof candidate.message === "string";
}

/** Normalises anything a command rejects with into an `AppError`. */
export function toAppError(error: unknown): AppError {
  if (isAppError(error)) return error;
  const message = error instanceof Error ? error.message : String(error);
  return { code: "internal", message };
}

/** Calls a backend command with its argument object; rejects with an `AppError`. */
export async function call<T>(command: string, args?: object): Promise<T> {
  try {
    return await invoke<T>(command, args === undefined ? undefined : { args });
  } catch (error) {
    // The IPC contract is that commands reject with a plain `AppError` object (docs/IPC.md).
    // eslint-disable-next-line @typescript-eslint/only-throw-error
    throw toAppError(error);
  }
}

/** Like `call`, with extra top-level parameters such as a progress `Channel`. */
export async function callWith<T>(
  command: string,
  args: object,
  extra: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, { args, ...extra });
  } catch (error) {
    // eslint-disable-next-line @typescript-eslint/only-throw-error
    throw toAppError(error);
  }
}
