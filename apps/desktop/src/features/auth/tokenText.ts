import type { OAuthStatus, OAuthToken } from "../../ipc";

/** Longest token name, in characters (as the backend checks). */
export const MAX_TOKEN_NAME = 100;

const time = (ms: number): string => new Date(ms).toLocaleTimeString();

/** When the token stops working, as of `nowMs`. */
export function expiry(token: OAuthToken, nowMs: number): string {
  if (token.expiresAtMs === null) return "no expiry given";
  if (token.expiresAtMs <= nowMs) return token.canRefresh ? "expired, renewed on send" : "expired";
  return `until ${time(token.expiresAtMs)}`;
}

/** One line about the token a request sends with. */
export function describeStatus(status: OAuthStatus): string {
  if (status.tokens.length === 0) return "No token yet.";
  const active = status.tokens.find((t) => t.active);
  if (active === undefined) return "No token in use: pick one or get a new one.";
  const until = active.expiresAtMs === null ? "" : `, until ${time(active.expiresAtMs)}`;
  return `Using “${active.name}”${until}${active.canRefresh ? " (renewed when it expires)" : ""}.`;
}

/** A name as typed, or why it cannot be used. */
export function checkName(name: string): { name: string } | { error: string } {
  const trimmed = name.trim();
  if (trimmed === "") return { error: "A token needs a name" };
  // Code points, as the backend counts them.
  if (Array.from(trimmed).length > MAX_TOKEN_NAME)
    return { error: `A token name has at most ${String(MAX_TOKEN_NAME)} characters` };
  return { name: trimmed };
}
