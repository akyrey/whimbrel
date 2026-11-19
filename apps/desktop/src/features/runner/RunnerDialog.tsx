import { Button, Modal } from "@roost/ui";
import { useMemo, useState } from "react";

import { type RunSummary, type StepResult, runCancel, runStart, toAppError } from "../../ipc";
import { ChecksView } from "../checks/ChecksView";
import { useCollections } from "../collections/collectionsStore";
import { activeEnvironment } from "../environments/environmentsStore";
import { type RunTarget, useRunner } from "./runnerStore";
import {
  AS_IS,
  EACH,
  type ModeChoice,
  type RunnableRequest,
  requestsUnder,
  stepsOf,
} from "./steps";

let runCount = 0;

function ModeSelect({
  request,
  value,
  onChange,
}: {
  request: RunnableRequest;
  value: ModeChoice;
  onChange: (v: ModeChoice) => void;
}) {
  return (
    <select
      aria-label={`How to run ${request.name}`}
      className="input"
      value={value}
      onChange={(e) => {
        onChange(e.target.value);
      }}
    >
      <option value={AS_IS}>As it is</option>
      {request.examples.length > 0 && (
        <option value={EACH}>Once per example ({String(request.examples.length)})</option>
      )}
      {request.examples.map((e) => (
        <option key={e.id} value={e.id}>
          With {e.name}
        </option>
      ))}
    </select>
  );
}

function Results({ results }: { results: StepResult[] }) {
  const [open, setOpen] = useState<number | null>(null);
  return (
    <table aria-label="Run results" className="w-full text-left text-sm">
      <thead className="text-xs text-zinc-500">
        <tr>
          <th>#</th>
          <th>Request</th>
          <th>Status</th>
          <th>Time</th>
          <th>Result</th>
        </tr>
      </thead>
      <tbody>
        {results.map((r) => (
          <tr
            key={r.index}
            className="cursor-pointer border-t border-zinc-800 align-top"
            onClick={() => {
              setOpen(open === r.index ? null : r.index);
            }}
          >
            <td>{String(r.index + 1)}</td>
            <td>
              <span className="font-mono text-xs text-zinc-500">{r.method}</span> {r.name}
              {r.example && <span className="text-zinc-400"> › {r.example}</span>}
              {open === r.index && r.checks && <ChecksView checks={r.checks} />}
            </td>
            <td>{r.status ?? "—"}</td>
            <td>{r.totalMs === null ? "—" : `${String(r.totalMs)} ms`}</td>
            <td className={r.passed ? "text-emerald-400" : "text-red-400"}>
              {r.passed ? "passed" : (r.error ?? "failed")}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

const summaryText = (s: RunSummary) =>
  `${String(s.passed)} passed, ${String(s.failed)} failed${s.skipped > 0 ? `, ${String(s.skipped)} not run` : ""}${s.cancelled ? " (cancelled)" : ""}`;

function Runner({ target }: { target: RunTarget }) {
  const view = useCollections((s) => s.views[target.root]);
  const requests = useMemo(
    () => requestsUnder(view?.collection.children ?? [], target.folder),
    [view, target.folder],
  );
  const [modes, setModes] = useState<Record<string, ModeChoice>>({});
  const [stop, setStop] = useState(true);
  const [results, setResults] = useState<StepResult[]>([]);
  const [runId, setRunId] = useState<string | null>(null);
  const [summary, setSummary] = useState<RunSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const environment = activeEnvironment(target.root);

  const start = async () => {
    runCount += 1;
    const id = `run-${String(runCount)}`;
    setRunId(id);
    setResults([]);
    setSummary(null);
    setError(null);
    try {
      const done = await runStart(
        {
          runId: id,
          root: target.root,
          steps: stepsOf(requests, modes),
          stopOnFailure: stop,
          ...(environment ? { environment: environment.id } : {}),
        },
        (result) => {
          setResults((previous) => [...previous, result]);
        },
      );
      setSummary(done);
    } catch (caught) {
      setError(toAppError(caught).message);
    } finally {
      setRunId(null);
    }
  };

  return (
    <Modal
      title={`Run ${target.name}`}
      onClose={useRunner.getState().close}
      footer={
        <>
          <Button onClick={useRunner.getState().close}>Close</Button>
          {runId !== null ? (
            <Button onClick={() => void runCancel(runId)}>Stop</Button>
          ) : (
            <Button variant="primary" disabled={requests.length === 0} onClick={() => void start()}>
              Run
            </Button>
          )}
        </>
      }
    >
      <div className="flex max-h-[70vh] w-[40rem] max-w-full flex-col gap-3 overflow-y-auto text-sm">
        {requests.length === 0 ? (
          <p className="text-zinc-400">There are no requests to run here.</p>
        ) : (
          <ol aria-label="Steps" className="flex flex-col gap-1">
            {requests.map((r) => (
              <li key={r.path} className="flex items-center gap-2">
                <span className="w-14 font-mono text-xs text-zinc-500">{r.method}</span>
                <span className="flex-1 truncate">{r.name}</span>
                <ModeSelect
                  request={r}
                  value={modes[r.path] ?? AS_IS}
                  onChange={(v) => {
                    setModes({ ...modes, [r.path]: v });
                  }}
                />
              </li>
            ))}
          </ol>
        )}
        <div className="flex items-center gap-4">
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={stop}
              onChange={(e) => {
                setStop(e.target.checked);
              }}
            />
            Stop on the first failure
          </label>
          <span className="text-xs text-zinc-500">Environment: {environment?.name ?? "none"}</span>
        </div>
        {error && (
          <p role="alert" className="text-red-400">
            {error}
          </p>
        )}
        {summary && <p aria-label="Run summary">{summaryText(summary)}</p>}
        {results.length > 0 && <Results results={results} />}
      </div>
    </Modal>
  );
}

/** The runner (FR-6.3), for the folder picked in the tree. */
export function RunnerDialog() {
  const target = useRunner((s) => s.target);
  return target ? <Runner key={`${target.root}:${target.folder}`} target={target} /> : null;
}
