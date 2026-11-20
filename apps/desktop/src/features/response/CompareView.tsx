import { useMemo } from "react";

import type { Response } from "../../ipc";
import {
  COMPARE_LIMIT,
  type ShownLine,
  comparableBody,
  diffBodies,
  headerChanges,
  withContext,
} from "./compare";

/** Unchanged lines kept around each change. */
const CONTEXT_LINES = 3;

const LINE_CLASS = {
  same: "text-zinc-400",
  added: "bg-emerald-950 text-emerald-300",
  removed: "bg-rose-950 text-rose-300",
} as const;

const MARK = { same: " ", added: "+", removed: "-" } as const;

const statusOf = (r: Response) => `${String(r.status)}${r.reason ? ` ${r.reason}` : ""}`;

function HeaderTable({ before, after }: { before: Response; after: Response }) {
  const changes = headerChanges(before.headers, after.headers);
  if (changes.length === 0) return <p className="text-zinc-500">Headers unchanged.</p>;
  return (
    <table aria-label="Header changes" className="w-full font-mono">
      <tbody>
        {changes.map((c) => (
          <tr key={c.name} className="align-top">
            <th scope="row" className="pr-4 text-left font-normal text-zinc-400">
              {c.name}
            </th>
            <td className="break-all pr-2 text-rose-300">{c.before ?? "—"}</td>
            <td className="break-all text-emerald-300">{c.after ?? "—"}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function Line({ line }: { line: ShownLine }) {
  if (line.kind === "skipped") {
    return (
      <li className="select-none py-0.5 text-zinc-500">
        ⋯ {line.count} unchanged {line.count === 1 ? "line" : "lines"}
      </li>
    );
  }
  return (
    <li
      data-change={line.kind}
      className={`whitespace-pre-wrap break-all ${LINE_CLASS[line.kind]}`}
    >
      <span aria-hidden="true" className="select-none pr-2">
        {MARK[line.kind]}
      </span>
      {line.text}
    </li>
  );
}

function BodyDiff({ before, after }: { before: string; after: string }) {
  const lines = useMemo(
    () =>
      before.length > COMPARE_LIMIT || after.length > COMPARE_LIMIT
        ? null
        : withContext(diffBodies(before, after), CONTEXT_LINES),
    [before, after],
  );
  if (lines === null) return <p className="text-zinc-500">Too large to compare line by line.</p>;
  if (lines.every((l) => l.kind === "skipped")) {
    return <p className="text-zinc-500">Body unchanged.</p>;
  }
  return (
    <ul aria-label="Body changes" className="font-mono">
      {lines.map((line, i) => (
        <Line key={i} line={line} />
      ))}
    </ul>
  );
}

/** The current response against the one before it (task 10.3). */
export function CompareView({ before, after }: { before: Response; after: Response }) {
  return (
    <div className="flex flex-col gap-3 text-xs">
      <p className="text-zinc-300">
        {statusOf(before)} → {statusOf(after)}
        <span className="ml-3 text-zinc-500">
          {before.timing.totalMs} ms → {after.timing.totalMs} ms
        </span>
      </p>
      <HeaderTable before={before} after={after} />
      <BodyDiff before={comparableBody(before)} after={comparableBody(after)} />
    </div>
  );
}
