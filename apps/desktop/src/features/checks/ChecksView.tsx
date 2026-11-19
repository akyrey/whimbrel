import type { Checks } from "../../ipc";

const MARK = { true: "✓", false: "✗" } as const;

/** The results of a send's assertions and captures (FR-6). */
export function ChecksView({ checks }: { checks: Checks }) {
  return (
    <ul aria-label="Checks" className="flex flex-col gap-1 text-sm">
      {checks.assertions.map((a, i) => (
        <li key={`a${String(i)}`} className={a.passed ? "text-emerald-400" : "text-red-400"}>
          {MARK[String(a.passed) as "true" | "false"]} {a.message}
        </li>
      ))}
      {checks.captures.map((c) => (
        <li key={`c${c.var}`} className={c.error === null ? "text-zinc-300" : "text-red-400"}>
          {c.error === null ? "✓" : "✗"} <span className="font-mono">{c.var}</span>
          {c.error !== null
            ? `: ${c.error}`
            : c.secret
              ? ` ← •••••• (secret${c.saved ? ", saved to the keychain" : ""})`
              : ` ← ${c.value ?? ""}`}
        </li>
      ))}
    </ul>
  );
}
