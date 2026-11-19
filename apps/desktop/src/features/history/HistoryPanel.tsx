import { Button, ConfirmDialog, toast } from "@roost/ui";
import { useEffect, useState } from "react";

import { type HistorySummary, historyClear, historyGet, historyList, toAppError } from "../../ipc";
import { NEW_REQUEST, type SendStatus } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";

const PAGE = 100;
const MASK = "••••••";

async function reopen(id: number) {
  try {
    const entry = await historyGet(id);
    const status: SendStatus =
      entry.outcome.kind === "completed"
        ? { kind: "done", response: entry.outcome.response, events: [], checks: null }
        : {
            kind: "failed",
            error: { code: entry.outcome.code, message: entry.outcome.message },
            events: [],
          };
    useTabs
      .getState()
      .openLoose({ ...NEW_REQUEST, name: entry.name, request: entry.request }, status);
    if (JSON.stringify(entry.request).includes(MASK)) {
      toast.info(
        "Secret values are masked in the history: put the {{variable}} back before sending.",
      );
    }
  } catch (caught) {
    toast.error(`Could not open it: ${toAppError(caught).message}`);
  }
}

const time = (ms: number) => new Date(ms).toLocaleString();

/** Sent requests, newest first, searchable (FR-8). */
export function HistoryPanel() {
  const [query, setQuery] = useState("");
  const [entries, setEntries] = useState<HistorySummary[]>([]);
  const [clearing, setClearing] = useState(false);
  /** Bumped to read the list again (after clearing it). */
  const [version, setVersion] = useState(0);
  useEffect(() => {
    let current = true;
    historyList(query, PAGE, 0).then(
      (found) => {
        if (current) setEntries(found);
      },
      (caught: unknown) => {
        toast.error(`Could not read the history: ${toAppError(caught).message}`);
      },
    );
    return () => {
      current = false;
    };
  }, [query, version]);

  return (
    <nav aria-label="History" className="flex min-h-0 flex-1 flex-col gap-2 p-2">
      <div className="flex gap-1">
        <input
          aria-label="Search the history"
          placeholder="Search"
          className="input min-w-0 flex-1 text-sm"
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
          }}
        />
        <Button
          onClick={() => {
            setClearing(true);
          }}
        >
          Clear
        </Button>
      </div>
      <ul className="min-h-0 flex-1 overflow-y-auto text-sm">
        {entries.length === 0 && <li className="text-xs text-zinc-500">Nothing sent yet.</li>}
        {entries.map((e) => (
          <li key={e.id}>
            <button
              type="button"
              className="flex w-full flex-col items-start px-1 py-1 text-left hover:bg-zinc-800"
              title={time(e.sentAtMs)}
              onClick={() => void reopen(e.id)}
            >
              <span className="flex w-full gap-2">
                <span className="w-12 shrink-0 font-mono text-[10px] text-zinc-400">
                  {e.method}
                </span>
                <span className="truncate text-zinc-200">{e.name}</span>
                <span
                  className={`ml-auto text-xs ${e.status === null ? "text-rose-400" : "text-zinc-400"}`}
                >
                  {e.status ?? e.errorCode}
                </span>
              </span>
              <span className="w-full truncate pl-14 text-xs text-zinc-500">{e.url}</span>
            </button>
          </li>
        ))}
      </ul>
      {clearing && (
        <ConfirmDialog
          title="Clear the history?"
          message="Every recorded request and response is deleted."
          confirmLabel="Clear"
          onCancel={() => {
            setClearing(false);
          }}
          onConfirm={() => {
            setClearing(false);
            void historyClear().then(() => {
              setVersion(version + 1);
            });
          }}
        />
      )}
    </nav>
  );
}
