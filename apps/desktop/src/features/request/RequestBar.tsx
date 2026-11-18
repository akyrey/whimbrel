import { Button } from "@roost/ui";

import { shortcutLabel } from "../shortcuts/registry";
import type { Tab } from "../tabs/model";
import { useState } from "react";

import { copyAsCurl } from "../curl/curlActions";
import { PreviewDialog } from "../send/PreviewDialog";
import { sendTab } from "../send/sendFlow";
import { useTabs } from "../tabs/tabsStore";
import { setMethod, setUrl } from "./editing";

/** Methods offered in the list; any other token can be typed (QUERY, PROPFIND, …). */
const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS", "QUERY"];

export const URL_INPUT_ID = "request-url";

/** Method, URL and Send/Cancel. */
export function RequestBar({ tab }: { tab: Tab }) {
  const { method, url } = tab.draft.request;
  const sending = tab.status.kind === "sending";
  const { edit, cancel } = useTabs.getState();
  const [previewing, setPreviewing] = useState(false);

  return (
    <form
      className="flex items-center gap-2"
      onSubmit={(e) => {
        e.preventDefault();
        sendTab(tab.id);
      }}
    >
      <input
        aria-label="Method"
        list="request-methods"
        className="input w-28 font-mono font-semibold uppercase"
        value={method}
        onChange={(e) => {
          edit(tab.id, setMethod(e.target.value));
        }}
      />
      <datalist id="request-methods">
        {METHODS.map((m) => (
          <option key={m} value={m} />
        ))}
      </datalist>
      <input
        id={URL_INPUT_ID}
        aria-label="URL"
        placeholder="https://api.example.com/users"
        className="input min-w-0 flex-1 font-mono"
        value={url}
        spellCheck={false}
        onChange={(e) => {
          edit(tab.id, setUrl(e.target.value));
        }}
      />
      {/* Distinct keys: if React reused one <button>, a click on Cancel that ends the request
          could still submit the form once the button turns back into Send. */}
      {sending ? (
        <Button key="cancel" onClick={() => void cancel(tab.id)} title={shortcutLabel("cancel")}>
          Cancel
        </Button>
      ) : (
        <Button key="send" type="submit" variant="primary" title={shortcutLabel("send")}>
          Send
        </Button>
      )}
      <Button
        onClick={() => {
          setPreviewing(true);
        }}
      >
        Preview
      </Button>
      <Button title="Secrets masked" onClick={() => void copyAsCurl(tab, false)}>
        Copy cURL
      </Button>
      <Button
        title="With the secret values: take care where you paste it"
        onClick={() => void copyAsCurl(tab, true)}
      >
        Copy cURL with secrets
      </Button>
      {previewing && (
        <PreviewDialog
          tab={tab}
          onClose={() => {
            setPreviewing(false);
          }}
        />
      )}
    </form>
  );
}
