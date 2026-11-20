import type { Timing } from "../../ipc";
import { type PhaseName, phasesOf } from "./timing";

const BAR_CLASS: Record<PhaseName, string> = {
  dns: "bg-violet-400",
  connect: "bg-amber-400",
  waiting: "bg-sky-400",
  download: "bg-emerald-400",
};

/** A bar's share of the whole, never so thin it disappears. */
const MIN_BAR_PERCENT = 0.5;

/** The response's time as a waterfall (task 10.2). */
export function TimingView({ timing }: { timing: Timing }) {
  const phases = phasesOf(timing);
  const total = Math.max(
    1,
    phases.reduce((sum, p) => sum + p.ms, 0),
  );
  const percent = (ms: number) => `${String((ms / total) * 100)}%`;
  return (
    <div className="flex flex-col gap-3 text-xs">
      <ul
        aria-label="Timing"
        className="grid grid-cols-[10rem_1fr_4rem] items-center gap-x-3 gap-y-2"
      >
        {phases.map((p) => (
          <li key={p.phase} className="contents">
            <span className="text-zinc-400">{p.label}</span>
            <span className="relative h-2 rounded bg-zinc-800">
              <span
                aria-hidden="true"
                className={`absolute inset-y-0 rounded ${BAR_CLASS[p.phase]}`}
                style={{
                  left: percent(p.startMs),
                  width: `max(${String(MIN_BAR_PERCENT)}%, ${percent(p.ms)})`,
                }}
              />
            </span>
            <span className="text-right font-mono text-zinc-200">{p.ms} ms</span>
          </li>
        ))}
      </ul>
      <p className="text-zinc-400">
        Total <span className="font-mono text-zinc-200">{timing.totalMs} ms</span>
        {timing.connectMs === null &&
          " · Reused a connection, so there was no DNS lookup or connect."}
        {timing.connectMs !== null &&
          timing.dnsMs === null &&
          " · The host is an address, so there was no DNS lookup."}
      </p>
    </div>
  );
}
