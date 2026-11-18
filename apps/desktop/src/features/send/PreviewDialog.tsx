import { Button, Modal, Skeleton } from "@roost/ui";
import { useEffect, useState } from "react";

import { type AppError, type Request, requestPreview, toAppError } from "../../ipc";
import type { Tab } from "../tabs/model";
import { sendArgs } from "../tabs/tabsStore";

type State =
  { kind: "loading" } | { kind: "ready"; request: Request } | { kind: "failed"; error: AppError };

function Missing({ error }: { error: AppError }) {
  const detail = error.detail as { undefined?: string[]; unsetSecrets?: string[] } | undefined;
  return (
    <div role="alert" className="flex flex-col gap-1 text-sm text-amber-200">
      {(detail?.undefined ?? []).length > 0 && <p>Not defined: {detail?.undefined?.join(", ")}</p>}
      {(detail?.unsetSecrets ?? []).length > 0 && (
        <p>No value in the keychain for: {detail?.unsetSecrets?.join(", ")}</p>
      )}
      {detail === undefined && <p>{error.message}</p>}
    </div>
  );
}

/** The request exactly as it would be sent, secrets masked (FR-2.5). */
export function PreviewDialog({ tab, onClose }: { tab: Tab; onClose: () => void }) {
  const [state, setState] = useState<State>({ kind: "loading" });
  useEffect(() => {
    requestPreview(sendArgs(tab)).then(
      (request) => {
        setState({ kind: "ready", request });
      },
      (caught: unknown) => {
        setState({ kind: "failed", error: toAppError(caught) });
      },
    );
  }, [tab]);

  return (
    <Modal
      title="Resolved request"
      onClose={onClose}
      footer={<Button onClick={onClose}>Close</Button>}
    >
      {state.kind === "loading" && <Skeleton rows={3} label="Resolving" />}
      {state.kind === "failed" && <Missing error={state.error} />}
      {state.kind === "ready" && (
        <pre
          aria-label="Request as sent"
          className="max-h-[60vh] overflow-auto whitespace-pre-wrap font-mono text-xs text-zinc-200"
        >
          {`${state.request.method} ${state.request.url}\n`}
          {state.request.headers
            .filter((h) => h.enabled)
            .map((h) => `${h.name}: ${h.value}\n`)
            .join("")}
          {state.request.body.type === "text" ? `\n${state.request.body.text}` : ""}
        </pre>
      )}
    </Modal>
  );
}
