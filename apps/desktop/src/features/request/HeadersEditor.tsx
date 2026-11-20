import { HeaderRows } from "../headers/HeaderRows";
import { InheritedHeaders } from "../headers/InheritedHeaders";
import type { Tab } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";
import { editHeaders } from "./editing";

/** The request's own headers, then those it inherits from its folders and collection. */
export function HeadersEditor({ tab }: { tab: Tab }) {
  const { headers } = tab.draft.request;
  return (
    <div>
      <HeaderRows
        headers={headers}
        onChange={(next) => {
          useTabs.getState().edit(
            tab.id,
            editHeaders(() => next),
          );
        }}
      />
      {tab.file && <InheritedHeaders file={tab.file} own={headers} />}
    </div>
  );
}
