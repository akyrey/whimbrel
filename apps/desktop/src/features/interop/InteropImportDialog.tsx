import { Button, Modal, toast } from "@roost/ui";
import { useState } from "react";

import {
  type ImportTarget,
  type InteropSummary,
  chooseFile,
  chooseFolder,
  interopImport,
  interopRead,
  toAppError,
} from "../../ipc";
import { useCollections } from "../collections/collectionsStore";
import { useEnvironments } from "../environments/environmentsStore";

const FILES = [{ name: "Postman or .http", extensions: ["json", "http", "rest"] }];
const NEW = "";

function Summary({ summary }: { summary: InteropSummary }) {
  return (
    <div aria-label="Import summary" className="flex flex-col gap-1 text-sm">
      <p className="font-medium text-zinc-100">{summary.name}</p>
      {summary.requests > 0 && (
        <p>
          {String(summary.requests)} requests
          {summary.folders.length > 0 ? ` in ${summary.folders.join(", ")}` : ""}
        </p>
      )}
      {summary.environments.length > 0 && <p>Environments: {summary.environments.join(", ")}</p>}
      {summary.variables > 0 && <p>{String(summary.variables)} collection variables</p>}
      {summary.warnings.length > 0 && (
        <ul className="ml-4 list-disc text-xs text-amber-300">
          {summary.warnings.map((w) => (
            <li key={w}>{w}</li>
          ))}
        </ul>
      )}
    </div>
  );
}

/** Import a Postman collection or environment, or a `.http` file (task 8.4). */
export function InteropImportDialog({ onClose }: { onClose: () => void }) {
  const roots = Object.values(useCollections((s) => s.views));
  const [path, setPath] = useState("");
  const [summary, setSummary] = useState<InteropSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [target, setTarget] = useState(NEW);
  const [busy, setBusy] = useState(false);

  const pick = async () => {
    const chosen = await chooseFile("Choose a Postman or .http file", FILES);
    if (chosen === null) return;
    setPath(chosen);
    setError(null);
    try {
      setSummary(await interopRead(chosen));
    } catch (caught) {
      setSummary(null);
      setError(toAppError(caught).message);
    }
  };
  const run = async () => {
    if (!summary) return;
    setBusy(true);
    try {
      let to: ImportTarget;
      if (target === NEW) {
        const folder = await chooseFolder("Choose a folder for the new collection");
        if (folder === null) return;
        to = { kind: "new", folder, name: summary.name };
      } else {
        to = { kind: "existing", root: target, folderName: summary.name };
      }
      const done = await interopImport({ path, target: to });
      if (to.kind === "new") await useCollections.getState().open(done.root);
      else await useCollections.getState().refresh(done.root);
      await useEnvironments.getState().load(done.root);
      const count = done.installed.requests.length;
      toast.success(
        count > 0
          ? `Imported ${String(count)} request${count === 1 ? "" : "s"}`
          : `Imported ${String(done.installed.environments.length)} environment(s)`,
      );
      onClose();
    } catch (caught) {
      setError(toAppError(caught).message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title="Import from Postman or .http"
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!summary || busy} onClick={() => void run()}>
            Import
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-3 text-sm">
        <div className="flex items-center gap-2">
          <Button onClick={() => void pick()}>Choose file…</Button>
          <span aria-label="Chosen file" className="truncate font-mono text-xs text-zinc-400">
            {path || "No file chosen"}
          </span>
        </div>
        <label className="flex items-center gap-2">
          Import into
          <select
            aria-label="Import into"
            className="input"
            value={target}
            onChange={(e) => {
              setTarget(e.target.value);
            }}
          >
            <option value={NEW}>A new collection…</option>
            {roots.map((v) => (
              <option key={v.root} value={v.root}>
                {v.collection.name}
              </option>
            ))}
          </select>
        </label>
        {error && (
          <p role="alert" className="text-red-400">
            {error}
          </p>
        )}
        {summary && <Summary summary={summary} />}
      </div>
    </Modal>
  );
}
