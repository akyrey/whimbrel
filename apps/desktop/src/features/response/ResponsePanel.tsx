import { Skeleton } from "@roost/ui";
import { useState } from "react";

import type { Response } from "../../ipc";
import type { SendStatus } from "../tabs/model";
import { BodyView } from "./BodyView";
import { EventsView } from "./EventsView";
import { TimingView } from "./TimingView";
import { ChecksView } from "../checks/ChecksView";
import { summary } from "../checks/checks";
import { cookiesOf, formatSize, statusTone } from "./format";

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
      <span className="text-zinc-400">{formatSize(body.size)}</span>
      <span className="text-zinc-500">{httpVersion}</span>
    </div>
  );
}

function Pairs({ label, pairs }: { label: string; pairs: [string, string][] }) {
  return (
    <dl aria-label={label} className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 font-mono text-xs">
      {pairs.map(([name, value], i) => (
        <div key={i} className="contents">
          <dt className="text-zinc-400">{name}</dt>
          <dd className="break-all text-zinc-200">{value}</dd>
        </div>
      ))}
    </dl>
  );
}

type View = "body" | "headers" | "cookies" | "events" | "checks" | "timing";

function Tabs({
  views,
  view,
  onView,
}: {
  views: [View, string][];
  view: View;
  onView: (v: View) => void;
}) {
  return (
    <div
      role="tablist"
      aria-label="Response parts"
      className="flex flex-wrap gap-x-4 border-b border-zinc-800 text-sm"
    >
      {views.map(([v, label]) => (
        <button
          key={v}
          type="button"
          role="tab"
          aria-selected={view === v}
          className={
            view === v
              ? "-mb-px border-b-2 border-sky-500 pb-1 text-zinc-100"
              : "pb-1 text-zinc-400 hover:text-zinc-200"
          }
          onClick={() => {
            onView(v);
          }}
        >
          {label}
        </button>
      ))}
    </div>
  );
}

export function ResponsePanel({ status }: { status: SendStatus }) {
  const [view, setView] = useState<View>("body");

  if (status.kind === "idle") {
    return <p className="p-4 text-sm text-zinc-500">Send a request to see its response.</p>;
  }
  if (status.kind === "sending") {
    return status.events.length > 0 ? (
      <section aria-label="Response" className="min-h-0 flex-1 overflow-auto p-4">
        <EventsView events={status.events} live />
      </section>
    ) : (
      <Skeleton rows={3} label="Waiting for the response" />
    );
  }
  if (status.kind === "failed") {
    return (
      <div role="alert" className="flex flex-col gap-2 p-4 text-sm text-rose-300">
        <p className="font-semibold">
          {status.error.code === "cancelled" ? "Cancelled" : "The request failed"}
        </p>
        <p>{status.error.message}</p>
        {status.events.length > 0 && <EventsView events={status.events} live={false} />}
      </div>
    );
  }
  const { response, events, checks } = status;
  const result = summary(checks);
  const cookies = cookiesOf(response.headers);
  const views: [View, string][] = [
    ["body", "Body"],
    ["headers", `Headers (${String(response.headers.length)})`],
    ...(cookies.length > 0
      ? [["cookies", `Cookies (${String(cookies.length)})`] as [View, string]]
      : []),
    ...(events.length > 0
      ? [["events", `Events (${String(events.length)})`] as [View, string]]
      : []),
    ...(result
      ? [["checks", `Checks (${String(result.passed)}/${String(result.total)})`] as [View, string]]
      : []),
    ["timing", "Timing"],
  ];
  return (
    <section aria-label="Response" className="flex min-h-0 flex-1 flex-col gap-2 p-4">
      <Summary response={response} />
      <Tabs views={views} view={view} onView={setView} />
      <div className="min-h-0 flex-1 overflow-auto">
        {view === "body" && <BodyView response={response} />}
        {view === "headers" && (
          <Pairs label="Response headers" pairs={response.headers.map((h) => [h.name, h.value])} />
        )}
        {view === "cookies" && (
          <Pairs
            label="Cookies set"
            pairs={cookies.map((c) => [
              c.name,
              c.attributes ? `${c.value}  (${c.attributes})` : c.value,
            ])}
          />
        )}
        {view === "events" && <EventsView events={events} live={false} />}
        {view === "checks" && checks && <ChecksView checks={checks} />}
        {view === "timing" && <TimingView timing={response.timing} />}
      </div>
    </section>
  );
}
