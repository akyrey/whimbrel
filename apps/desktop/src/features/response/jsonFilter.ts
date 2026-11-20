import { useEffect, useState } from "react";

import { responseSelect, toAppError } from "../../ipc";

/** Waits for a pause in typing before running the path. */
const FILTER_DELAY_MS = 200;

export type Filtered = { text: string | null; error: string | null };

const NONE: Filtered = { text: null, error: null };

/** The matches of `path` in the JSON `body` as pretty JSON, or why the path cannot run
 * (task 9.4). A blank path filters nothing. */
export function useJsonFilter(body: string, path: string): Filtered {
  const [filtered, setFiltered] = useState<Filtered>(NONE);
  const blank = path.trim() === "";

  useEffect(() => {
    if (blank) return;
    let current = true;
    const timer = setTimeout(() => {
      responseSelect(body, path).then(
        (text) => {
          if (current) setFiltered({ text, error: null });
        },
        (caught: unknown) => {
          if (current) setFiltered({ text: null, error: toAppError(caught).message });
        },
      );
    }, FILTER_DELAY_MS);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [body, path, blank]);

  return blank ? NONE : filtered;
}
