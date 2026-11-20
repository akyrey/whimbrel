import { Button, toast } from "@roost/ui";
import { useEffect, useState } from "react";

import {
  type DeviceCode,
  type OAuthStatus,
  type TokenChange,
  oauthAuthorize,
  oauthStatus,
  oauthTokenChange,
  requestCancel,
  toAppError,
} from "../../ipc";
import type { Tab } from "../tabs/model";
import { sendArgs } from "../tabs/tabsStore";
import { useAuthVersion } from "./authVersion";
import { TokensDialog } from "./TokensDialog";
import { MAX_TOKEN_NAME, checkName, describeStatus } from "./tokenText";

type Busy = { device: DeviceCode | null } | null;

/** The OAuth 2 tokens of the request's effective auth: get a named one, see the one in use,
 * forget it, or manage every kept token. */
export function OAuthPanel({ tab }: { tab: Tab }) {
  const [status, setStatus] = useState<OAuthStatus | null>(null);
  const [busy, setBusy] = useState<Busy>(null);
  const [version, setVersion] = useState(0);
  const [newName, setNewName] = useState("");
  const [managing, setManaging] = useState(false);
  const inherited = useAuthVersion((s) => s.version);
  const args = sendArgs(tab);
  const key = JSON.stringify([args.root, args.path, args.auth, args.environment]);

  useEffect(() => {
    let current = true;
    oauthStatus(args).then(
      (s) => {
        if (current) setStatus(s);
      },
      () => {
        if (current) setStatus(null);
      },
    );
    return () => {
      current = false;
    };
    // The arguments are read through `key`, which changes with them.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, version, inherited]);

  if (!status?.configured) return null;

  const active = status.tokens.find((t) => t.active);

  const change = async (request: TokenChange) => {
    try {
      setStatus(await oauthTokenChange(args, request));
    } catch (caught) {
      toast.error(`Could not change the token: ${toAppError(caught).message}`);
      setVersion((v) => v + 1);
      throw caught;
    }
  };

  const getToken = async () => {
    const checked = newName.trim() === "" ? null : checkName(newName);
    if (checked !== null && "error" in checked) {
      toast.error(checked.error);
      return;
    }
    setBusy({ device: null });
    try {
      await oauthAuthorize(args, checked?.name ?? null, (device) => {
        setBusy({ device });
      });
      setNewName("");
      toast.success("Signed in");
    } catch (caught) {
      const error = toAppError(caught);
      if (error.code !== "cancelled") toast.error(`Could not get a token: ${error.message}`);
    } finally {
      setBusy(null);
      setVersion((v) => v + 1);
    }
  };

  return (
    <div
      role="group"
      aria-label="OAuth 2 token"
      className="flex flex-wrap items-center gap-2 rounded bg-zinc-900 px-3 py-2 text-sm"
    >
      <span className="text-zinc-300">{describeStatus(status)}</span>
      {busy === null ? (
        <>
          <input
            aria-label="New token name"
            placeholder="Name (optional)"
            value={newName}
            maxLength={MAX_TOKEN_NAME}
            className="w-40 rounded bg-zinc-800 px-2 py-1 text-zinc-100"
            onChange={(event) => {
              setNewName(event.target.value);
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter") void getToken();
            }}
          />
          <Button variant="primary" onClick={() => void getToken()}>
            {status.tokens.length > 0 ? "Get a new token" : "Get token"}
          </Button>
          {active !== undefined && (
            <Button
              onClick={() => void change({ type: "remove", id: active.id }).catch(() => undefined)}
            >
              Forget token
            </Button>
          )}
          {status.tokens.length > 0 && (
            <Button
              onClick={() => {
                setManaging(true);
              }}
            >
              {`Manage tokens (${String(status.tokens.length)})`}
            </Button>
          )}
        </>
      ) : (
        <>
          <span role="status" className="text-sky-300">
            {busy.device === null
              ? status.grant === "authorization_code"
                ? "Waiting for the browser…"
                : "Signing in…"
              : "Enter this code on the page that opened:"}
          </span>
          {busy.device !== null && (
            <>
              <code
                aria-label="Device code"
                className="rounded bg-zinc-800 px-2 py-0.5 text-base tracking-widest text-zinc-100"
              >
                {busy.device.userCode}
              </code>
              <span className="text-xs text-zinc-500">{busy.device.verificationUri}</span>
            </>
          )}
          <Button onClick={() => void requestCancel(`oauth:${tab.id}`)}>Cancel</Button>
        </>
      )}
      {managing && (
        <TokensDialog
          tokens={status.tokens}
          onChange={change}
          onError={(message) => {
            toast.error(message);
          }}
          onClose={() => {
            setManaging(false);
          }}
        />
      )}
    </div>
  );
}
