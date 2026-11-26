import type { Auth, OAuth2Config } from "../../ipc";

export type AuthType = Auth["type"];

const EMPTY_OAUTH: OAuth2Config = {
  grant: "client_credentials",
  tokenUrl: "",
  authUrl: "",
  deviceUrl: "",
  clientId: "",
  clientSecret: "",
  scope: "",
  redirectUri: "",
};

/** The default fields of a newly picked type. */
export function authOfType(type: AuthType): Auth {
  switch (type) {
    case "inherit":
      return { type: "inherit" };
    case "none":
      return { type: "none" };
    case "basic":
      return { type: "basic", username: "", password: "" };
    case "bearer":
      return { type: "bearer", token: "" };
    case "api_key":
      return { type: "api_key", name: "", value: "", location: "header" };
    case "oauth2":
      return { type: "oauth2", ...EMPTY_OAUTH };
  }
}
