import { Button } from "@roost/ui";
import { useState } from "react";

import { type Validation, specValidate, toAppError } from "../../ipc";
import type { Tab } from "../tabs/model";
import { operationOf, useSpecs } from "./specsStore";

function Result({ result }: { result: Validation }) {
  const where = [result.response, result.mediaType].filter(Boolean).join(" ");
  if (!result.checked) return <span className="text-zinc-400">{result.note}</span>;
  if (result.errors.length === 0) {
    return <span className="text-emerald-400">✓ Matches the spec ({where})</span>;
  }
  return (
    <div className="text-rose-400">
      ✗ {String(result.errors.length)} difference{result.errors.length === 1 ? "" : "s"} from the
      spec ({where})
      <ul aria-label="Schema errors" className="ml-4 list-disc text-xs">
        {result.errors.map((e, i) => (
          <li key={i}>
            <span className="font-mono">{e.pointer || "/"}</span> {e.message}
          </li>
        ))}
      </ul>
    </div>
  );
}

/** "Validate against the spec" for a generated request's last response (FR-7.6). */
export function SchemaCheck({ tab }: { tab: Tab }) {
  const byRoot = useSpecs((s) => s.byRoot);
  const [result, setResult] = useState<{ for: unknown; value: Validation | string } | null>(null);
  const link = tab.file ? operationOf(byRoot, tab.file.root, tab.file.path) : null;
  if (!tab.file || !link || tab.status.kind !== "done") return null;
  const { response } = tab.status;
  const root = tab.file.root;
  const run = async () => {
    try {
      const value = await specValidate({
        root,
        dir: link.dir,
        operationId: link.id,
        status: response.status,
        body: response.body.text ?? "",
        ...(response.body.contentType ? { contentType: response.body.contentType } : {}),
      });
      setResult({ for: response, value });
    } catch (caught) {
      setResult({ for: response, value: toAppError(caught).message });
    }
  };
  const shown = result?.for === response ? result.value : null;
  return (
    <section aria-label="Spec validation" className="flex flex-col gap-1 px-4 text-sm">
      <div>
        <Button onClick={() => void run()}>Validate against the spec</Button>
      </div>
      {typeof shown === "string" && (
        <span role="alert" className="text-rose-400">
          {shown}
        </span>
      )}
      {shown !== null && typeof shown !== "string" && <Result result={shown} />}
    </section>
  );
}
