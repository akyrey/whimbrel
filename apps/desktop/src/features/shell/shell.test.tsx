import { useToasts } from "@roost/ui";
import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "../../App";
import { installMockBackend } from "../../test/mockBackend";
import { useCollections } from "../collections/collectionsStore";
import { useEnvironments } from "../environments/environmentsStore";
import { useLayout } from "../layout/layoutStore";
import { useRunner } from "../runner/runnerStore";
import { DEFAULT_SETTINGS, useSettings } from "../settings/settingsStore";
import { sendArgs, useTabs } from "../tabs/tabsStore";
import { useDialogs } from "./dialogsStore";

beforeEach(() => {
  useToasts.setState({ toasts: [] });
  localStorage.clear();
  installMockBackend();
  useTabs.setState({ tabs: [], activeId: null });
  useCollections.setState({ views: {} });
  useEnvironments.setState({ entries: {}, active: {} });
  useRunner.setState({ target: null });
  useDialogs.setState({ open: null });
  useSettings.setState(DEFAULT_SETTINGS);
});

afterEach(async () => {
  cleanup();
  await new Promise((resolve) => setTimeout(resolve, 0));
  clearMocks();
});

async function command(text: string) {
  fireEvent.keyDown(window, { key: "k", ctrlKey: true });
  const palette = screen.getByRole("dialog", { name: "Command palette" });
  await userEvent.type(within(palette).getByRole("combobox"), `${text}{Enter}`);
}

it("settings change the theme, the editor font and the default timeout", async () => {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Settings" }));
  const dialog = screen.getByRole("dialog", { name: "Settings" });
  await userEvent.selectOptions(within(dialog).getByRole("combobox", { name: "Theme" }), "light");
  await userEvent.selectOptions(
    within(dialog).getByRole("combobox", { name: "Editor font size" }),
    "16",
  );
  await userEvent.type(
    within(dialog).getByRole("spinbutton", { name: "Default timeout in milliseconds" }),
    "5000",
  );
  await userEvent.selectOptions(
    within(dialog).getByRole("combobox", { name: "Response placement" }),
    "beside",
  );
  expect(useLayout.getState().split).toBe("beside");
  await command("response below");
  expect(useLayout.getState().split).toBe("below");
  await userEvent.click(within(dialog).getByRole("button", { name: "Done" }));

  expect(useSettings.getState()).toMatchObject({ theme: "light", fontSize: 16, timeoutMs: 5000 });
  expect(JSON.parse(localStorage.getItem("whimbrel.settings") ?? "{}")).toMatchObject({
    fontSize: 16,
  });
  useTabs.getState().newScratch();
  const tab = useTabs.getState().tabs[0];
  expect(tab && sendArgs(tab).settings?.timeoutMs).toBe(5000);
});

it("the palette opens dialogs, runs collections and switches environments", async () => {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open…" }));
  await screen.findByRole("region", { name: "Demo API" });

  await command("import an openapi");
  expect(screen.getByRole("dialog", { name: "Import OpenAPI" })).toBeInTheDocument();
  useDialogs.getState().close();
  await command("from postman");
  expect(screen.getByRole("dialog", { name: "Import from Postman or .http" })).toBeInTheDocument();
  useDialogs.getState().close();
  await command("a curl command");
  expect(screen.getByRole("dialog", { name: "Import cURL" })).toBeInTheDocument();
  useDialogs.getState().close();

  await command("use the dark theme");
  expect(useSettings.getState().theme).toBe("dark");
  await command("use production in demo");
  expect(useEnvironments.getState().active["/demo"]).toBe("production");
  await command("run demo api");
  expect(screen.getByRole("dialog", { name: "Run Demo API" })).toBeInTheDocument();
});
