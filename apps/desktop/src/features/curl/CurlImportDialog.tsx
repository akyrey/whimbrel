import { Button, Modal } from "@roost/ui";
import { useState } from "react";

import { importCurl } from "./curlActions";

/** Paste a cURL command to open it as a request (FR-2.6). */
export function CurlImportDialog({ onClose }: { onClose: () => void }) {
  const [text, setText] = useState("");
  return (
    <Modal
      title="Import cURL"
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button
            variant="primary"
            disabled={text.trim() === ""}
            onClick={() => {
              void importCurl(text).then((ok) => {
                if (ok) onClose();
              });
            }}
          >
            Import
          </Button>
        </>
      }
    >
      <textarea
        aria-label="cURL command"
        autoFocus
        spellCheck={false}
        placeholder="curl 'https://api.example.com/users' -H 'Accept: application/json'"
        className="input h-40 w-full font-mono text-xs"
        value={text}
        onChange={(e) => {
          setText(e.target.value);
        }}
      />
    </Modal>
  );
}
