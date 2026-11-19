import { Button, Modal } from "@roost/ui";
import { useState } from "react";

import { type SpecSource, type SpecSummary, chooseFile, specRead, toAppError } from "../../ipc";
import { useCollections } from "../collections/collectionsStore";
import { type TargetChoice, importSpec } from "./specActions";
import { useSpecs } from "./specsStore";

const SPEC_FILES = [{ name: "OpenAPI", extensions: ["yaml", "yml", "json"] }];
const NEW = "";

function SummaryView({ summary }: { summary: SpecSummary }) {
  return (
    <div aria-label="Import summary" className="flex flex-col gap-1 text-sm">
      <p className="font-medium text-zinc-100">
        {summary.title}{" "}
        {summary.apiVersion && <span className="text-zinc-400">v{summary.apiVersion}</span>}
        <span className="ml-2 text-xs text-zinc-500">OpenAPI {summary.versionText}</span>
      </p>
      <p>
        {String(summary.operations)} requests in {summary.folders.join(", ") || "the top level"}
        {summary.webhooks > 0 && `; ${String(summary.webhooks)} webhooks (shown in the spec only)`}
      </p>
      <p>Environments: {summary.environments.join(", ")}</p>
      {summary.warnings.length > 0 && (
        <details>
          <summary className="cursor-pointer text-amber-300">
            {String(summary.warnings.length)} warning{summary.warnings.length === 1 ? "" : "s"}
          </summary>
          <ul className="ml-4 list-disc text-xs text-zinc-400">
            {summary.warnings.map((w) => (
              <li key={w}>{w}</li>
            ))}
          </ul>
        </details>
      )}
    </div>
  );
}

/** Import an OpenAPI document from a file or a URL, after a summary (FR-6). */
export function SpecImportDialog({ onClose }: { onClose: () => void }) {
  const roots = Object.values(useCollections((s) => s.views));
  const [kind, setKind] = useState<"file" | "url">("file");
  const [path, setPath] = useState("");
  const [url, setUrl] = useState("");
  const [allowRemote, setAllowRemote] = useState(false);
  const [summary, setSummary] = useState<SpecSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [target, setTarget] = useState(NEW);

  const source: SpecSource = kind === "file" ? { kind: "file", path } : { kind: "url", url };
  const ready = kind === "file" ? path !== "" : url.trim() !== "";
  const reset = () => {
    setSummary(null);
    setError(null);
  };
  const read = async (remote: boolean) => {
    setBusy(true);
    setError(null);
    try {
      setSummary(await specRead({ source, allowRemote: remote }));
      setAllowRemote(remote);
    } catch (caught) {
      setSummary(null);
      setError(toAppError(caught).message);
    } finally {
      setBusy(false);
    }
  };
  const run = async () => {
    if (!summary) return;
    const choice: TargetChoice =
      target === NEW
        ? { kind: "new", name: summary.title }
        : { kind: "existing", root: target, folderName: summary.title };
    setBusy(true);
    const done = await importSpec(source, allowRemote, choice);
    setBusy(false);
    if (!done) return;
    const spec = useSpecs
      .getState()
      .byRoot[done.root]?.find((s) => s.dir === done.installed.specDir);
    if (spec) void useSpecs.getState().show({ root: done.root, dir: spec.dir });
    onClose();
  };

  return (
    <Modal
      title="Import OpenAPI"
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          {summary ? (
            <Button variant="primary" disabled={busy} onClick={() => void run()}>
              Import
            </Button>
          ) : (
            <Button variant="primary" disabled={!ready || busy} onClick={() => void read(false)}>
              Read
            </Button>
          )}
        </>
      }
    >
      <div className="flex flex-col gap-3 text-sm">
        <div role="radiogroup" aria-label="Source" className="flex gap-4">
          {(["file", "url"] as const).map((k) => (
            <label key={k} className="flex items-center gap-2">
              <input
                type="radio"
                name="spec-source"
                checked={kind === k}
                onChange={() => {
                  setKind(k);
                  reset();
                }}
              />
              {k === "file" ? "File" : "URL"}
            </label>
          ))}
        </div>
        {kind === "file" ? (
          <div className="flex items-center gap-2">
            <Button
              onClick={() =>
                void chooseFile("Choose an OpenAPI document", SPEC_FILES).then((chosen) => {
                  if (chosen !== null) {
                    setPath(chosen);
                    reset();
                  }
                })
              }
            >
              Choose file…
            </Button>
            <span aria-label="Chosen file" className="truncate font-mono text-xs text-zinc-400">
              {path || "No file chosen"}
            </span>
          </div>
        ) : (
          <input
            aria-label="Spec URL"
            className="input font-mono"
            placeholder="https://api.example.com/openapi.yaml"
            value={url}
            onChange={(e) => {
              setUrl(e.target.value);
              reset();
            }}
          />
        )}
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
                {v.collection.name} (a new folder)
              </option>
            ))}
          </select>
        </label>
        {error && (
          <p role="alert" className="text-red-400">
            {error}
          </p>
        )}
        {summary && <SummaryView summary={summary} />}
        {summary && summary.remote.length > 0 && !allowRemote && (
          <div
            role="alert"
            className="flex flex-col gap-1 rounded border border-amber-700 p-2 text-xs"
          >
            <p>The spec refers to documents on other servers, which were not fetched:</p>
            <ul className="ml-4 list-disc font-mono">
              {summary.remote.map((r) => (
                <li key={r}>{r}</li>
              ))}
            </ul>
            <div>
              <Button disabled={busy} onClick={() => void read(true)}>
                Fetch them
              </Button>
            </div>
          </div>
        )}
      </div>
    </Modal>
  );
}
