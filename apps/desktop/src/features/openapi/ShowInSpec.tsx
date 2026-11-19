import { Button, toast } from "@roost/ui";

import { toAppError } from "../../ipc";
import type { Tab } from "../tabs/model";
import { operationOf, useSpecs } from "./specsStore";

/** "Show in spec" for a request generated from a spec. */
export function ShowInSpec({ tab }: { tab: Tab }) {
  const byRoot = useSpecs((s) => s.byRoot);
  const link = tab.file ? operationOf(byRoot, tab.file.root, tab.file.path) : null;
  if (!tab.file || !link) return null;
  const root = tab.file.root;
  return (
    <div className="flex justify-end">
      <Button
        onClick={() => {
          useSpecs
            .getState()
            .show({ root, dir: link.dir }, { kind: "operation", id: link.id })
            .catch((caught: unknown) => {
              toast.error(`Could not open the spec: ${toAppError(caught).message}`);
            });
        }}
      >
        Show in spec
      </Button>
    </div>
  );
}
