import { AuthEditor } from "../auth/AuthEditor";
import { OAuthPanel } from "../auth/OAuthPanel";
import type { Tab } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";

/** The request's own auth, and its OAuth 2 token when that is what applies. */
export function AuthPanel({ tab }: { tab: Tab }) {
  return (
    <div className="flex flex-col gap-2">
      <AuthEditor
        auth={tab.draft.auth}
        allowInherit
        onChange={(auth) => {
          useTabs.getState().edit(tab.id, (d) => ({ ...d, auth }));
        }}
      />
      <OAuthPanel tab={tab} />
    </div>
  );
}
