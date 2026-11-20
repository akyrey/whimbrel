import type { OAuthStatus, OAuthToken } from "../../ipc";
import { MAX_TOKEN_NAME, checkName, describeStatus, expiry } from "./tokenText";

const token = (over: Partial<OAuthToken> = {}): OAuthToken => ({
  id: "a",
  name: "Alice",
  obtainedAtMs: 0,
  expiresAtMs: null,
  scope: null,
  canRefresh: false,
  active: false,
  ...over,
});

const status = (tokens: OAuthToken[]): OAuthStatus => ({
  configured: true,
  grant: "client_credentials",
  hasToken: tokens.some((t) => t.active),
  expiresAtMs: null,
  scope: null,
  canRefresh: false,
  tokens,
});

describe("token text", () => {
  it("says when a token stops working", () => {
    expect(expiry(token(), 10)).toBe("no expiry given");
    expect(expiry(token({ expiresAtMs: 5 }), 10)).toBe("expired");
    expect(expiry(token({ expiresAtMs: 5, canRefresh: true }), 10)).toBe(
      "expired, renewed on send",
    );
    expect(expiry(token({ expiresAtMs: 20 }), 10)).toMatch(/^until /);
  });

  it("describes no tokens, none in use, and the one in use", () => {
    expect(describeStatus(status([]))).toBe("No token yet.");
    expect(describeStatus(status([token()]))).toMatch(/^No token in use/);
    expect(describeStatus(status([token({ active: true, canRefresh: true })]))).toBe(
      "Using “Alice” (renewed when it expires).",
    );
  });

  it("trims names and refuses empty or long ones", () => {
    expect(checkName("  Ops ")).toEqual({ name: "Ops" });
    expect(checkName("  ")).toEqual({ error: "A token needs a name" });
    expect(checkName("x".repeat(MAX_TOKEN_NAME + 1))).toHaveProperty("error");
    expect(checkName("é".repeat(MAX_TOKEN_NAME))).toHaveProperty("name");
  });
});
