import { listen } from "@tauri-apps/api/event";

import type { CollectionChanged } from "./generated";

/** Calls `handler` for every change to an open collection made outside the app (and the app's
 * own saves, recognised by hash). Resolves to the function that stops listening. */
export function onCollectionChanged(
  handler: (changed: CollectionChanged) => void,
): Promise<() => void> {
  return listen<CollectionChanged>("collection_changed", (event) => {
    handler(event.payload);
  });
}
