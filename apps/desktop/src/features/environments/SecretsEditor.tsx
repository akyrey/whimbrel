import { Button } from "@roost/ui";
import { useState } from "react";

import type { SecretName, SecretState } from "../../ipc";

type Props = {
  secrets: SecretName[];
  states: SecretState[];
  onChange: (secrets: SecretName[]) => void;
  /** Sends the value to the keychain; nothing reads it back. */
  onSet: (name: string, value: string) => Promise<void>;
  onClear: (name: string) => Promise<void>;
};

/** Secret names (in the file) and whether the keychain holds each one's value. */
export function SecretsEditor({ secrets, states, onChange, onSet, onClear }: Props) {
  const [values, setValues] = useState<Record<string, string>>({});
  const isSet = (name: string) => states.find((s) => s.name === name)?.set ?? false;
  const saved = (name: string) => states.some((s) => s.name === name);
  return (
    <fieldset className="flex flex-col gap-1 text-sm">
      <legend className="mb-1 text-xs font-semibold uppercase tracking-wide text-zinc-400">
        Secrets (values in the keychain)
      </legend>
      {secrets.map((secret, index) => (
        <div key={index} className="flex items-center gap-2">
          <input
            aria-label={`Secret ${String(index + 1)} name`}
            placeholder="Name"
            className="input w-36 font-mono"
            value={secret.name}
            onChange={(e) => {
              onChange(secrets.map((s, i) => (i === index ? { ...s, name: e.target.value } : s)));
            }}
          />
          <input
            aria-label={`Secret ${String(index + 1)} description`}
            placeholder="Where to find it"
            className="input min-w-0 flex-1"
            value={secret.description}
            onChange={(e) => {
              onChange(
                secrets.map((s, i) => (i === index ? { ...s, description: e.target.value } : s)),
              );
            }}
          />
          {saved(secret.name) ? (
            <>
              <span
                className={`text-xs ${isSet(secret.name) ? "text-emerald-400" : "text-amber-300"}`}
              >
                {isSet(secret.name) ? "set" : "not set"}
              </span>
              <input
                type="password"
                autoComplete="off"
                aria-label={`Value of ${secret.name}`}
                className="input w-32"
                value={values[secret.name] ?? ""}
                onChange={(e) => {
                  setValues({ ...values, [secret.name]: e.target.value });
                }}
              />
              <Button
                disabled={(values[secret.name] ?? "") === ""}
                onClick={() => {
                  void onSet(secret.name, values[secret.name] ?? "").then(() => {
                    setValues({ ...values, [secret.name]: "" });
                  });
                }}
              >
                Set
              </Button>
              {isSet(secret.name) && (
                <Button
                  aria-label={`Clear ${secret.name}`}
                  onClick={() => void onClear(secret.name)}
                >
                  Clear
                </Button>
              )}
            </>
          ) : (
            <span className="text-xs text-zinc-500">save to set a value</span>
          )}
          <Button
            aria-label={`Remove secret ${String(index + 1)}`}
            onClick={() => {
              onChange(secrets.filter((_, i) => i !== index));
            }}
          >
            ✕
          </Button>
        </div>
      ))}
      <div>
        <Button
          onClick={() => {
            onChange([...secrets, { name: "", description: "" }]);
          }}
        >
          Add secret
        </Button>
      </div>
    </fieldset>
  );
}
