import { Button, Modal } from "@roost/ui";
import { useState } from "react";

import type { OAuthToken, TokenChange } from "../../ipc";
import { MAX_TOKEN_NAME, checkName, expiry } from "./tokenText";

type Props = {
  tokens: OAuthToken[];
  /** Applies a change; rejects (after telling the user) when it fails. */
  onChange: (change: TokenChange) => Promise<void>;
  onError: (message: string) => void;
  onClose: () => void;
};

type RowProps = Omit<Props, "tokens" | "onClose"> & { token: OAuthToken; nowMs: number };

function TokenRow({ token, nowMs, onChange, onError }: RowProps) {
  const [name, setName] = useState(token.name);

  const commit = () => {
    if (name === token.name) return;
    const checked = checkName(name);
    if ("error" in checked) {
      onError(checked.error);
      setName(token.name);
      return;
    }
    setName(checked.name);
    onChange({ type: "rename", id: token.id, name: checked.name }).catch(() => {
      setName(token.name);
    });
  };

  const details = [
    `got ${new Date(token.obtainedAtMs).toLocaleString()}`,
    expiry(token, nowMs),
    token.scope === null ? null : `scope ${token.scope}`,
  ].filter((part) => part !== null);

  return (
    <li className="flex items-center gap-3 rounded bg-zinc-800/60 px-3 py-2 text-sm">
      <input
        type="radio"
        name="oauth-token-in-use"
        aria-label={`Use ${token.name}`}
        checked={token.active}
        onChange={() => void onChange({ type: "select", id: token.id }).catch(() => undefined)}
      />
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <input
          aria-label={`Name of ${token.name}`}
          value={name}
          maxLength={MAX_TOKEN_NAME}
          className="rounded bg-zinc-900 px-2 py-1 text-zinc-100"
          onChange={(event) => {
            setName(event.target.value);
          }}
          onBlur={commit}
          onKeyDown={(event) => {
            if (event.key === "Enter") commit();
            if (event.key === "Escape") {
              event.stopPropagation();
              setName(token.name);
            }
          }}
        />
        <span className="text-xs text-zinc-500">{details.join(" · ")}</span>
      </div>
      <Button
        aria-label={`Delete ${token.name}`}
        onClick={() => void onChange({ type: "remove", id: token.id }).catch(() => undefined)}
      >
        Delete
      </Button>
    </li>
  );
}

/** The tokens kept for one OAuth 2 configuration: pick the one in use, rename, delete. */
export function TokensDialog({ tokens, onChange, onError, onClose }: Props) {
  const [nowMs] = useState(() => Date.now());
  return (
    <Modal
      title="OAuth 2 tokens"
      onClose={onClose}
      footer={<Button onClick={onClose}>Close</Button>}
    >
      {tokens.length === 0 ? (
        <p className="text-sm text-zinc-400">No tokens kept for this configuration.</p>
      ) : (
        <ul aria-label="Tokens" className="flex max-h-[60vh] flex-col gap-2 overflow-y-auto">
          {tokens.map((token) => (
            <TokenRow
              key={token.id}
              token={token}
              nowMs={nowMs}
              onChange={onChange}
              onError={onError}
            />
          ))}
        </ul>
      )}
    </Modal>
  );
}
