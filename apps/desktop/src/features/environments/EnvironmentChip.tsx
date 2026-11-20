import { useSidebar } from "../layout/sidebarStore";
import { environmentPickerId, useEnvironments } from "./environmentsStore";

/** Where the request goes: the collection's active environment, protected ones in rose (task 9.6).
 * Clicking it focuses the picker in the sidebar. */
export function EnvironmentChip({ root }: { root: string | undefined }) {
  const entry = useEnvironments((s) =>
    root === undefined ? undefined : s.entries[root]?.find((e) => e.id === s.active[root]),
  );
  if (root === undefined) return null;
  const name = entry ? (entry.name ?? entry.id) : "No environment";
  const tone = entry?.protected
    ? "border-rose-700 text-rose-300"
    : entry
      ? "border-zinc-700 text-zinc-200"
      : "border-zinc-800 text-zinc-500";
  return (
    <button
      type="button"
      aria-label={`Environment: ${name}`}
      title={entry?.protected ? "Protected: sending asks first" : "Change the environment"}
      className={`max-w-40 shrink-0 truncate rounded-full border px-2 py-0.5 text-xs ${tone}`}
      onClick={() => {
        useSidebar.getState().show("collections");
        setTimeout(() => {
          document.getElementById(environmentPickerId(root))?.focus();
        }, 0);
      }}
    >
      {entry?.protected ? `🔒 ${name}` : name}
    </button>
  );
}
