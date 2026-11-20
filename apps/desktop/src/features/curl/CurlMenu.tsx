import { Button, ContextMenu } from "@roost/ui";
import { useState } from "react";

import type { Tab } from "../tabs/model";
import { copyAsCurl } from "./curlActions";

type Anchor = { x: number; y: number };

/** Gap in pixels between the button and the menu below it. */
const MENU_GAP = 4;

/** The rarely used copy actions, folded into one button so the request bar stays narrow. */
export function CurlMenu({ tab }: { tab: Tab }) {
  const [anchor, setAnchor] = useState<Anchor | null>(null);
  return (
    <>
      <Button
        aria-haspopup="menu"
        aria-expanded={anchor !== null}
        title="Copy the request as cURL"
        onClick={(e) => {
          const box = e.currentTarget.getBoundingClientRect();
          setAnchor({ x: box.left, y: box.bottom + MENU_GAP });
        }}
      >
        cURL <span aria-hidden="true">▾</span>
      </Button>
      {anchor && (
        <ContextMenu
          label="Copy as cURL"
          x={anchor.x}
          y={anchor.y}
          onClose={() => {
            setAnchor(null);
          }}
          items={[
            { label: "Copy cURL", onSelect: () => void copyAsCurl(tab, false) },
            {
              label: "Copy cURL with secrets",
              destructive: true,
              onSelect: () => void copyAsCurl(tab, true),
            },
          ]}
        />
      )}
    </>
  );
}
