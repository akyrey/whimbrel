import { toast } from "@roost/ui";

import { toAppError } from "../../ipc";
import { useSpecs } from "./specsStore";

/** The specs imported into the open collections; a click opens the browser. */
export function SpecList() {
  const byRoot = useSpecs((s) => s.byRoot);
  const entries = Object.entries(byRoot).flatMap(([root, specs]) =>
    specs.map((spec) => ({ root, spec })),
  );
  if (entries.length === 0) return null;
  return (
    <nav aria-label="API specs" className="border-t border-zinc-800 p-2 text-sm">
      <h2 className="mb-1 text-xs uppercase text-zinc-500">API specs</h2>
      <ul>
        {entries.map(({ root, spec }) => (
          <li key={`${root}:${spec.dir}`}>
            <button
              type="button"
              className="w-full truncate text-left text-zinc-300 hover:text-zinc-100"
              onClick={() => {
                useSpecs
                  .getState()
                  .show({ root, dir: spec.dir })
                  .catch((caught: unknown) => {
                    toast.error(`Could not open the spec: ${toAppError(caught).message}`);
                  });
              }}
            >
              {spec.title}
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
}
