/** Every action as a palette command (FR-9.1). */
import type { PaletteItem } from "@roost/palette";

import { historyClear } from "../../ipc";
import { closeCollection, exportHttp, openCollection } from "../collections/actions";
import { useCollections } from "../collections/collectionsStore";
import { useEnvironments } from "../environments/environmentsStore";
import { useSpecs } from "../openapi/specsStore";
import { useRunner } from "../runner/runnerStore";
import { useSettings } from "../settings/settingsStore";
import { useTabs } from "../tabs/tabsStore";
import { useDialogs } from "./dialogsStore";

const item = (id: string, group: string, label: string, run: () => void): PaletteItem => ({
  id,
  group,
  label,
  run,
});

function exampleItems(send: (id: string) => void): PaletteItem[] {
  const { tabs, activeId } = useTabs.getState();
  const tab = tabs.find((t) => t.id === activeId);
  if (!tab) return [];
  return tab.draft.examples.map((example) =>
    item(`example:${example.id}`, "Examples", `Send ${tab.draft.name} › ${example.name}`, () => {
      useTabs.getState().pickExample(tab.id, example.id);
      send(tab.id);
    }),
  );
}

function collectionItems(): PaletteItem[] {
  return Object.values(useCollections.getState().views).flatMap((view) => {
    const { root } = view;
    const name = view.collection.name;
    const entries = useEnvironments.getState().entries[root] ?? [];
    return [
      item(`run:${root}`, "Collections", `Run ${name}…`, () => {
        useRunner.getState().open({ root, folder: "", name });
      }),
      item(
        `export:${root}`,
        "Collections",
        `Export ${name} as .http…`,
        () => void exportHttp(root, "", name),
      ),
      item(`close:${root}`, "Collections", `Close ${name}`, () => void closeCollection(root)),
      ...entries
        .filter((e) => e.error === null)
        .map((e) =>
          item(`env:${root}:${e.id}`, "Environments", `Use ${e.name ?? e.id} in ${name}`, () => {
            useEnvironments.getState().setActive(root, e.id);
          }),
        ),
      ...(useSpecs.getState().byRoot[root] ?? []).map((spec) =>
        item(
          `spec:${root}:${spec.dir}`,
          "API specs",
          `Show the ${spec.title} spec`,
          () => void useSpecs.getState().show({ root, dir: spec.dir }),
        ),
      ),
    ];
  });
}

/** App-wide commands beyond the shortcut ones. */
export function appItems(send: (id: string) => void): PaletteItem[] {
  const dialogs = useDialogs.getState();
  const settings = useSettings.getState();
  const dark = settings.theme === "dark";
  return [
    item("open-collection", "Collections", "Open a collection", () => void openCollection()),
    item("import-curl", "Import", "Import a cURL command…", () => {
      dialogs.show("curl");
    }),
    item("import-openapi", "Import", "Import an OpenAPI document…", () => {
      dialogs.show("openapi");
    }),
    item("import-postman", "Import", "Import from Postman or .http…", () => {
      dialogs.show("interop");
    }),
    item("settings", "Settings", "Settings…", () => {
      dialogs.show("settings");
    }),
    item("theme", "Settings", dark ? "Use the light theme" : "Use the dark theme", () => {
      settings.change({ theme: dark ? "light" : "dark" });
    }),
    item("clear-history", "History", "Clear the history", () => void historyClear()),
    ...collectionItems(),
    ...exampleItems(send),
  ];
}
