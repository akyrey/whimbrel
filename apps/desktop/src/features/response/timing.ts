import type { Timing } from "../../ipc";

export type PhaseName = "dns" | "connect" | "waiting" | "download";

export type Phase = { phase: PhaseName; label: string; startMs: number; ms: number };

const LABELS: Record<PhaseName, string> = {
  dns: "DNS",
  connect: "Connect (TCP + TLS)",
  waiting: "Waiting",
  download: "Download",
};

/** Where a response's time went, in order (task 10.2). DNS and connect are there only when the
 * send opened a connection (D-H20); waiting runs to the headers, download to the end. Each is
 * whole milliseconds, so the parts are clamped to never run backwards. */
export function phasesOf(timing: Timing): Phase[] {
  const durations: [PhaseName, number | null][] = [
    ["dns", timing.dnsMs],
    ["connect", timing.connectMs],
  ];
  const opened = durations.flatMap(([phase, ms]) => (ms === null ? [] : [{ phase, ms }]));
  const openedMs = opened.reduce((sum, p) => sum + p.ms, 0);
  const all = [
    ...opened,
    { phase: "waiting" as const, ms: Math.max(0, timing.headersMs - openedMs) },
    { phase: "download" as const, ms: Math.max(0, timing.totalMs - timing.headersMs) },
  ];
  return all.map((p, i) => ({
    phase: p.phase,
    label: LABELS[p.phase],
    startMs: all.slice(0, i).reduce((sum, q) => sum + q.ms, 0),
    ms: p.ms,
  }));
}
