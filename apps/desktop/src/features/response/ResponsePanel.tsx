import { Skeleton } from "@roost/ui";
import { Suspense, lazy, useState } from "react";

import type { Response } from "../../ipc";
import type { SendStatus } from "../tabs/model";
import { formatSize, showBody, statusTone } from "./format";

const ReadOnlyCode = lazy(async () => ({ default: (await import("@roost/editor")).ReadOnlyCode }));

const TONE_CLASS = {
  ok: "text-emerald-400",
  redirect: "text-sky-300",
  client: "text-amber-300",
  server: "text-rose-400",
} as const;

function Summary({ response }: { response: Response }) {
  const { status, reason, timing, body, httpVersion } = response;
  return (
    <div className="flex items-center gap-4 text-sm">
      <span className={`font-semibold ${TONE_CLASS[statusTone(status)]}`}>
        {status} {reason ?? ""}
      </span>
      <span className="text-zinc-400" title={`Headers after ${String(timing.headersMs)} ms`}>
        {timing.totalMs} ms
      </span>
      <span className="text-zinc-400">
        {formatSize(body.size)}
        {body.truncated ? " (cut short)" : ""}
      </span>
      <span className="text-zinc-500">{httpVersion}</span>
    </div>
  );
}

function BodyView({ response }: { response: Response }) {
  const shown = showBody(response.body);
  if (shown.kind === "empty") return <p className="text-sm text-zinc-500">No body.</p>;
  if (shown.kind === "binary") {
    return <p className="text-sm text-zinc-500">Binary body, {formatSize(shown.size)}.</p>;
  }
  return (
    <Suspense fallback={<Skeleton rows={6} label="Loading the body" />}>
      <ReadOnlyCode text={shown.text} label="Response body" language={shown.language} wrap />
    </Suspense>
  );
}

function HeadersView({ response }: { response: Response }) {
  return (
    <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 font-mono text-xs">
      {response.headers.map((h, i) => (
        <div key={i} className="contents">
          <dt className="text-zinc-400">{h.name}</dt>
          <dd className="break-all text-zinc-200">{h.value}</dd>
        </div>
      ))}
    </dl>
  );
}

type View = "body" | "headers";

export function ResponsePanel({ status }: { status: SendStatus }) {
  const [view, setView] = useState<View>("body");

  if (status.kind === "idle") {
    return <p className="p-4 text-sm text-zinc-500">Send a request to see its response.</p>;
  }
  if (status.kind === "sending") return <Skeleton rows={3} label="Waiting for the response" />;
  if (status.kind === "failed") {
    return (
      <div role="alert" className="p-4 text-sm text-rose-300">
        <p className="font-semibold">
          {status.error.code === "cancelled" ? "Cancelled" : "The request failed"}
        </p>
        <p>{status.error.message}</p>
      </div>
    );
  }
  const { response } = status;
  return (
    <section aria-label="Response" className="flex min-h-0 flex-1 flex-col gap-2 p-4">
      <Summary response={response} />
      <div role="tablist" aria-label="Response parts" className="flex gap-3 text-sm">
        {(["body", "headers"] as const).map((v) => (
          <button
            key={v}
            type="button"
            role="tab"
            aria-selected={view === v}
            className={view === v ? "text-zinc-100 underline" : "text-zinc-400"}
            onClick={() => {
              setView(v);
            }}
          >
            {v === "body" ? "Body" : `Headers (${String(response.headers.length)})`}
          </button>
        ))}
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        {view === "body" ? <BodyView response={response} /> : <HeadersView response={response} />}
      </div>
    </section>
  );
}
