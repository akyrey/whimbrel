import { Button, Modal, toast } from "@roost/ui";
import { useState } from "react";

import {
  type Change,
  type Choice,
  type Field,
  type Plan,
  type SpecSource,
  type StoredSpec,
  specReimportApply,
  specReimportPlan,
  toAppError,
} from "../../ipc";
import { useCollections } from "../collections/collectionsStore";
import { specKey, useSpecs } from "./specsStore";

type Choices = Record<string, Partial<Record<Field, Choice>>>;

const LABELS: Record<Change["kind"], string> = {
  added: "New",
  removed: "Left the spec (kept, marked)",
  updated: "Regenerated (not edited)",
  merged: "Merged with your edits",
};

const short = (value: unknown): string => {
  const text = typeof value === "string" ? value : JSON.stringify(value);
  return text.length > 80 ? `${text.slice(0, 77)}…` : text;
};

function sourceOf(spec: StoredSpec): SpecSource {
  return /^https?:\/\//i.test(spec.source)
    ? { kind: "url", url: spec.source }
    : { kind: "file", path: spec.source };
}

function ChangeRow({
  change,
  choices,
  choose,
}: {
  change: Change;
  choices: Choices;
  choose: (id: string, field: Field, c: Choice) => void;
}) {
  return (
    <li className="flex flex-col gap-1">
      <span>
        <span className="font-mono">{change.operationId}</span>{" "}
        <span className="text-zinc-400">{LABELS[change.kind]}</span>
      </span>
      {change.kind === "merged" &&
        change.conflicts.map((c) => (
          <fieldset
            key={c.field}
            aria-label={`${change.operationId} ${c.field}`}
            className="ml-4 flex flex-col gap-1 text-xs"
          >
            <legend className="text-amber-300">{c.field}: changed by you and by the spec</legend>
            {(["mine", "theirs"] as const).map((side) => (
              <label key={side} className="flex items-center gap-2">
                <input
                  type="radio"
                  name={`${change.operationId}-${c.field}`}
                  checked={(choices[change.operationId]?.[c.field] ?? "mine") === side}
                  onChange={() => {
                    choose(change.operationId, c.field, side);
                  }}
                />
                {side === "mine" ? "Keep mine" : "Take the spec's"}:{" "}
                <span className="font-mono">{short(side === "mine" ? c.current : c.new)}</span>
              </label>
            ))}
          </fieldset>
        ))}
    </li>
  );
}

/** Re-import a kept spec from its source, or another (FR-7.5). */
export function ReimportDialog({
  root,
  spec,
  onClose,
}: {
  root: string;
  spec: StoredSpec;
  onClose: () => void;
}) {
  const initial = sourceOf(spec);
  const [location, setLocation] = useState(initial.kind === "url" ? initial.url : initial.path);
  const [plan, setPlan] = useState<Plan | null>(null);
  const [choices, setChoices] = useState<Choices>({});
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const source: SpecSource = /^https?:\/\//i.test(location)
    ? { kind: "url", url: location }
    : { kind: "file", path: location };
  const base = { root, dir: spec.dir, source, allowRemote: false };

  const compare = async () => {
    setBusy(true);
    setError(null);
    try {
      setPlan(await specReimportPlan(base));
    } catch (caught) {
      setError(toAppError(caught).message);
    } finally {
      setBusy(false);
    }
  };
  const run = async () => {
    setBusy(true);
    try {
      const done = await specReimportApply({ ...base, choices });
      await useCollections.getState().refresh(root);
      await useSpecs.getState().load(root);
      const ref = { root, dir: spec.dir };
      useSpecs.setState((s) => ({
        apis: Object.fromEntries(Object.entries(s.apis).filter(([k]) => k !== specKey(ref))),
      }));
      await useSpecs.getState().show(ref);
      toast.success(
        `Re-imported: ${String(done.changes.length)} change${done.changes.length === 1 ? "" : "s"}`,
      );
      onClose();
    } catch (caught) {
      setError(toAppError(caught).message);
      setBusy(false);
    }
  };

  return (
    <Modal
      title={`Re-import ${spec.title}`}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          {plan && plan.changes.length > 0 ? (
            <Button variant="primary" disabled={busy} onClick={() => void run()}>
              Apply
            </Button>
          ) : (
            <Button
              variant="primary"
              disabled={busy || location.trim() === ""}
              onClick={() => void compare()}
            >
              Compare
            </Button>
          )}
        </>
      }
    >
      <div className="flex max-h-[70vh] w-[36rem] max-w-full flex-col gap-3 overflow-y-auto text-sm">
        <input
          aria-label="New spec location"
          className="input font-mono"
          value={location}
          onChange={(e) => {
            setLocation(e.target.value);
            setPlan(null);
          }}
        />
        {error && (
          <p role="alert" className="text-rose-400">
            {error}
          </p>
        )}
        {plan && plan.changes.length === 0 && <p>Nothing changed since the last import.</p>}
        {plan && plan.changes.length > 0 && (
          <ul aria-label="Re-import changes" className="flex flex-col gap-2">
            {plan.changes.map((c) => (
              <ChangeRow
                key={`${c.kind}:${c.operationId}`}
                change={c}
                choices={choices}
                choose={(id, field, choice) => {
                  setChoices({ ...choices, [id]: { ...choices[id], [field]: choice } });
                }}
              />
            ))}
          </ul>
        )}
        {plan && plan.skipped.length > 0 && (
          <p className="text-xs text-zinc-400">
            Deleted by hand, left alone: {plan.skipped.join(", ")}
          </p>
        )}
      </div>
    </Modal>
  );
}
