import type { StreamEvent } from "../../ipc";

/** Events of a stream, as they arrived (FR-3.4). */
export function EventsView({ events, live }: { events: StreamEvent[]; live: boolean }) {
  return (
    <div className="flex flex-col gap-1">
      {live && (
        <p role="status" className="text-xs text-sky-300">
          Receiving… {events.length} so far
        </p>
      )}
      <ol aria-label="Stream events" className="flex flex-col gap-1 font-mono text-xs">
        {events.map((e) => (
          <li key={e.index} className="rounded bg-zinc-900 px-2 py-1">
            <span className="text-zinc-500">#{e.index}</span>
            {e.event !== null && <span className="ml-2 text-violet-300">{e.event}</span>}
            {e.id !== null && <span className="ml-2 text-zinc-500">id {e.id}</span>}
            <pre className="whitespace-pre-wrap text-zinc-200">{e.data}</pre>
          </li>
        ))}
      </ol>
    </div>
  );
}
