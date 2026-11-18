import { Button } from "@roost/ui";

import { shortcutLabel } from "../shortcuts/registry";
import { useRequest } from "./requestStore";

/** Methods offered in the list; any other token can be typed (QUERY, PROPFIND, …). */
const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS", "QUERY"];

export const URL_INPUT_ID = "request-url";

/** Method, URL and Send/Cancel. */
export function RequestBar() {
  const method = useRequest((s) => s.request.method);
  const url = useRequest((s) => s.request.url);
  const sending = useRequest((s) => s.status.kind === "sending");
  const { setMethod, setUrl, send, cancel } = useRequest.getState();

  return (
    <form
      className="flex items-center gap-2"
      onSubmit={(e) => {
        e.preventDefault();
        void send();
      }}
    >
      <input
        aria-label="Method"
        list="request-methods"
        className="input w-28 font-mono font-semibold uppercase"
        value={method}
        onChange={(e) => {
          setMethod(e.target.value);
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
          setUrl(e.target.value);
        }}
      />
      {/* Distinct keys: if React reused one <button>, a click on Cancel that ends the request
          could still submit the form once the button turns back into Send. */}
      {sending ? (
        <Button key="cancel" onClick={() => void cancel()} title={shortcutLabel("cancel")}>
          Cancel
        </Button>
      ) : (
        <Button key="send" type="submit" variant="primary" title={shortcutLabel("send")}>
          Send
        </Button>
      )}
    </form>
  );
}
