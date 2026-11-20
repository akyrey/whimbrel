/** The variables in scope for the active tab, fetched when what decides them changes. */
import { createContext, useContext, useEffect, useState } from "react";

import { type ScopeVariable, variablesInScope } from "../../ipc";
import { useEnvironments } from "../environments/environmentsStore";
import type { Tab } from "../tabs/model";
import { sendArgs } from "../tabs/tabsStore";

export const VariablesContext = createContext<ScopeVariable[]>([]);

export const useVariables = (): ScopeVariable[] => useContext(VariablesContext);

/** Fetches the variables for `tab`; failures leave the list empty (hints only). */
export function useScopeVariables(tab: Tab): ScopeVariable[] {
  const args = sendArgs(tab);
  const revision = useEnvironments((s) => s.revision);
  const key = JSON.stringify([
    args.root,
    args.environment,
    args.example?.id,
    args.variables,
    tab.status.kind,
    revision,
  ]);
  const [variables, setVariables] = useState<ScopeVariable[]>([]);
  useEffect(() => {
    let current = true;
    variablesInScope(args)
      .then((list) => {
        if (current) setVariables(list);
      })
      .catch(() => {
        if (current) setVariables([]);
      });
    return () => {
      current = false;
    };
    // `key` stands for the parts of `args` that matter.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return variables;
}
