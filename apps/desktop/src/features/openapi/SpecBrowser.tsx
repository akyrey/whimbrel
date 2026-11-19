import { Button } from "@roost/ui";
import { useState } from "react";

import type { Api } from "../../ipc";
import { OperationPage } from "./OperationPage";
import { ReimportDialog } from "./ReimportDialog";
import { SchemaTree } from "./SchemaTree";
import { SpecNav } from "./SpecNav";
import { type SpecView, specKey, useSpecs } from "./specsStore";

function Overview({ api }: { api: Api }) {
  return (
    <article aria-label="Overview" className="flex flex-col gap-3 text-sm">
      {api.description && <p className="whitespace-pre-wrap text-zinc-300">{api.description}</p>}
      <section>
        <h3 className="font-medium">Servers</h3>
        <ul className="ml-4 list-disc font-mono text-xs">
          {api.servers.map((s) => (
            <li key={s.url}>
              {s.url}{" "}
              {s.description && <span className="font-sans text-zinc-400">— {s.description}</span>}
            </li>
          ))}
        </ul>
      </section>
      {api.securitySchemes.length > 0 && (
        <section>
          <h3 className="font-medium">Security</h3>
          <ul className="ml-4 list-disc">
            {api.securitySchemes.map((s) => (
              <li key={s.name}>
                <span className="font-mono">{s.name}</span>{" "}
                <span className="text-zinc-400">{s.kind.type.replaceAll("_", " ")}</span>
                {s.deprecated && <span className="ml-1 text-xs text-amber-300">deprecated</span>}
              </li>
            ))}
          </ul>
        </section>
      )}
      {api.warnings.length > 0 && (
        <section>
          <h3 className="font-medium text-amber-300">Warnings</h3>
          <ul className="ml-4 list-disc text-xs text-zinc-400">
            {api.warnings.map((w) => (
              <li key={w}>{w}</li>
            ))}
          </ul>
        </section>
      )}
    </article>
  );
}

function Detail({ api, view, files }: { api: Api; view: SpecView; files: Record<string, string> }) {
  const { focus } = view;
  if (focus.kind === "schema") {
    return <SchemaTree schema={api.schemas[focus.name]} label={focus.name} />;
  }
  if (focus.kind === "operation" || focus.kind === "webhook") {
    const list = focus.kind === "operation" ? api.operations : api.webhooks;
    const op = list.find((o) => o.id === focus.id);
    if (!op)
      return <p className="text-sm text-zinc-400">This operation is no longer in the spec.</p>;
    const file = focus.kind === "operation" ? (files[op.id] ?? null) : null;
    return (
      <OperationPage
        operation={op}
        root={view.root}
        file={file}
        readOnly={focus.kind === "webhook"}
      />
    );
  }
  return <Overview api={api} />;
}

/** The read-only spec view (OPENAPI.md "Spec browser"). */
export function SpecBrowser({ view }: { view: SpecView }) {
  const api = useSpecs((s) => s.apis[specKey(view)]);
  const spec = useSpecs((s) => s.byRoot[view.root]?.find((x) => x.dir === view.dir));
  const [reimporting, setReimporting] = useState(false);
  if (!api) return null;
  return (
    <section aria-label={`Spec ${api.title}`} className="flex min-h-0 flex-1 flex-col">
      <header className="flex items-center gap-3 border-b border-zinc-800 px-4 py-2">
        <h1 className="font-medium">{api.title}</h1>
        <span className="text-xs text-zinc-500">
          {api.apiVersion && `v${api.apiVersion} · `}OpenAPI {api.versionText}
        </span>
        {spec && (
          <Button
            className="ml-auto"
            onClick={() => {
              setReimporting(true);
            }}
          >
            Re-import…
          </Button>
        )}
        <Button className={spec ? "" : "ml-auto"} onClick={useSpecs.getState().close}>
          Close spec
        </Button>
      </header>
      {reimporting && spec && (
        <ReimportDialog
          root={view.root}
          spec={spec}
          onClose={() => {
            setReimporting(false);
          }}
        />
      )}
      <div className="flex min-h-0 flex-1">
        <SpecNav api={api} focus={view.focus} />
        <div className="min-w-0 flex-1 overflow-y-auto p-4">
          <Detail api={api} view={view} files={spec?.operations ?? {}} />
        </div>
      </div>
    </section>
  );
}
