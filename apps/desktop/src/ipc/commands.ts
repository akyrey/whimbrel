import type { Request, Response } from "./generated";
import { call } from "./invoke";

/** Sends `request` for the tab `requestId`; rejects with an `AppError` (code `cancelled` when
 * `requestCancel` stops it). */
export const requestSend = (requestId: string, request: Request): Promise<Response> =>
  call("request_send", { requestId, request });

/** Stops the tab's request; `false` when it had already finished. */
export const requestCancel = (requestId: string): Promise<boolean> =>
  call("request_cancel", { requestId });
