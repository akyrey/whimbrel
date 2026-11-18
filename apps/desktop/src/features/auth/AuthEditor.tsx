import type { Auth, Grant, KeyLocation, OAuth2Config } from "../../ipc";
import { type AuthType, authOfType } from "./authTypes";

const TYPES: { value: AuthType; label: string }[] = [
  { value: "inherit", label: "Inherit" },
  { value: "none", label: "No auth" },
  { value: "basic", label: "Basic" },
  { value: "bearer", label: "Bearer token" },
  { value: "api_key", label: "API key" },
  { value: "oauth2", label: "OAuth 2" },
];

const GRANTS: { value: Grant; label: string }[] = [
  { value: "client_credentials", label: "Client credentials" },
  { value: "authorization_code", label: "Authorization code (browser, PKCE)" },
  { value: "device_code", label: "Device code" },
];

type FieldProps = {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
};

function Field({ label, value, onChange, placeholder }: FieldProps) {
  return (
    <label className="flex items-center gap-2 text-sm">
      <span className="w-32 shrink-0 text-zinc-400">{label}</span>
      <input
        aria-label={label}
        className="input min-w-0 flex-1 font-mono"
        placeholder={placeholder}
        value={value}
        spellCheck={false}
        onChange={(e) => {
          onChange(e.target.value);
        }}
      />
    </label>
  );
}

const SECRET_HINT = "{{secretName}} keeps it in the keychain";

function OAuthFields({
  config,
  onChange,
}: {
  config: OAuth2Config;
  onChange: (c: OAuth2Config) => void;
}) {
  const field = (key: keyof OAuth2Config, label: string, placeholder?: string) => (
    <Field
      label={label}
      value={config[key]}
      placeholder={placeholder}
      onChange={(value) => {
        onChange({ ...config, [key]: value });
      }}
    />
  );
  return (
    <>
      <label className="flex items-center gap-2 text-sm">
        <span className="w-32 shrink-0 text-zinc-400">Grant</span>
        <select
          aria-label="Grant"
          className="input"
          value={config.grant}
          onChange={(e) => {
            onChange({ ...config, grant: e.target.value as Grant });
          }}
        >
          {GRANTS.map((g) => (
            <option key={g.value} value={g.value}>
              {g.label}
            </option>
          ))}
        </select>
      </label>
      {config.grant === "authorization_code" && field("authUrl", "Authorization URL")}
      {config.grant === "device_code" && field("deviceUrl", "Device URL")}
      {field("tokenUrl", "Token URL")}
      {field("clientId", "Client ID")}
      {field("clientSecret", "Client secret", `Empty for public clients; ${SECRET_HINT}`)}
      {field("scope", "Scope", "space separated")}
    </>
  );
}

type Props = { auth: Auth; onChange: (auth: Auth) => void; allowInherit: boolean };

/** Edits an auth (FR-5.1, FR-5.2); a request may inherit, the collection may not. */
export function AuthEditor({ auth, onChange, allowInherit }: Props) {
  const types = TYPES.filter((t) => allowInherit || t.value !== "inherit");
  return (
    <div className="flex flex-col gap-2">
      <select
        aria-label="Auth type"
        className="input w-48"
        value={auth.type}
        onChange={(e) => {
          onChange(authOfType(e.target.value as AuthType));
        }}
      >
        {types.map((t) => (
          <option key={t.value} value={t.value}>
            {t.label}
          </option>
        ))}
      </select>
      {auth.type === "inherit" && (
        <p className="text-xs text-zinc-500">
          Uses the auth of the nearest folder, else the collection's.
        </p>
      )}
      {auth.type === "basic" && (
        <>
          <Field
            label="Username"
            value={auth.username}
            onChange={(username) => {
              onChange({ ...auth, username });
            }}
          />
          <Field
            label="Password"
            value={auth.password}
            placeholder={SECRET_HINT}
            onChange={(password) => {
              onChange({ ...auth, password });
            }}
          />
        </>
      )}
      {auth.type === "bearer" && (
        <Field
          label="Token"
          value={auth.token}
          placeholder={SECRET_HINT}
          onChange={(token) => {
            onChange({ ...auth, token });
          }}
        />
      )}
      {auth.type === "api_key" && (
        <>
          <Field
            label="Name"
            value={auth.name}
            placeholder="X-Api-Key"
            onChange={(name) => {
              onChange({ ...auth, name });
            }}
          />
          <Field
            label="Value"
            value={auth.value}
            placeholder={SECRET_HINT}
            onChange={(value) => {
              onChange({ ...auth, value });
            }}
          />
          <label className="flex items-center gap-2 text-sm">
            <span className="w-32 shrink-0 text-zinc-400">Sent in</span>
            <select
              aria-label="Sent in"
              className="input"
              value={auth.location}
              onChange={(e) => {
                onChange({ ...auth, location: e.target.value as KeyLocation });
              }}
            >
              <option value="header">a header</option>
              <option value="query">the query</option>
            </select>
          </label>
        </>
      )}
      {auth.type === "oauth2" && (
        <OAuthFields
          config={auth}
          onChange={(config) => {
            onChange({ type: "oauth2", ...config });
          }}
        />
      )}
    </div>
  );
}
