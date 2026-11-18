import { AuthEditor } from "../auth/AuthEditor";
import { OAuthPanel } from "../auth/OAuthPanel";
import type { Tab } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";

/** The request's own auth, and its OAuth 2 token when that is what applies. */
export function AuthPanel({ tab }: { tab: Tab }) {
  return (
    <details className="text-sm" open={tab.draft.auth.type !== "inherit"}>
      <summary className="cursor-pointer text-zinc-400">Auth</summary>
      <div className="mt-2 flex flex-col gap-2">
        <AuthEditor
          auth={tab.draft.auth}
          allowInherit
          onChange={(auth) => {
            useTabs.getState().edit(tab.id, (d) => ({ ...d, auth }));
          }}
        />
        <OAuthPanel tab={tab} />
      </div>
    </details>
  );
}
