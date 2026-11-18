import { Button, toast } from "@roost/ui";
import { useEffect, useState } from "react";

import {
  type DeviceCode,
  type OAuthStatus,
  oauthAuthorize,
  oauthForget,
  oauthStatus,
  requestCancel,
  toAppError,
} from "../../ipc";
import type { Tab } from "../tabs/model";
import { sendArgs } from "../tabs/tabsStore";
import { useAuthVersion } from "./authVersion";

type Busy = { device: DeviceCode | null } | null;

function describe(status: OAuthStatus): string {
  if (!status.hasToken) return "No token yet.";
  const until =
    status.expiresAtMs === null
      ? ""
      : `, until ${new Date(status.expiresAtMs).toLocaleTimeString()}`;
  return `Token ready${until}${status.canRefresh ? " (renewed when it expires)" : ""}.`;
}

/** The OAuth 2 token of the request's effective auth: get one, see its state, forget it. */
export function OAuthPanel({ tab }: { tab: Tab }) {
  const [status, setStatus] = useState<OAuthStatus | null>(null);
  const [busy, setBusy] = useState<Busy>(null);
  const [version, setVersion] = useState(0);
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

  const getToken = async () => {
    setBusy({ device: null });
    try {
      await oauthAuthorize(args, (device) => {
        setBusy({ device });
      });
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
      <span className="text-zinc-300">{describe(status)}</span>
      {busy === null ? (
        <>
          <Button variant="primary" onClick={() => void getToken()}>
            {status.hasToken ? "Get a new token" : "Get token"}
          </Button>
          {status.hasToken && (
            <Button
              onClick={() =>
                void oauthForget(args).then(() => {
                  setVersion((v) => v + 1);
                })
              }
            >
              Forget token
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
    </div>
  );
}
