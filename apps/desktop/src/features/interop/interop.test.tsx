import { useToasts } from "@roost/ui";
import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "../../App";
import { installMockBackend } from "../../test/mockBackend";
import { useCollections } from "../collections/collectionsStore";
import { useTabs } from "../tabs/tabsStore";

beforeEach(() => {
  useToasts.setState({ toasts: [] });
  localStorage.clear();
  installMockBackend();
  useTabs.setState({ tabs: [], activeId: null });
  useCollections.setState({ views: {} });
});

afterEach(async () => {
  cleanup();
  await new Promise((resolve) => setTimeout(resolve, 0));
  clearMocks();
});

async function openDemo() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open…" }));
  return screen.findByRole("region", { name: "Demo API" });
}

it("imports a Postman collection into a folder of the open collection", async () => {
  const tree = await openDemo();
  await userEvent.click(screen.getByRole("button", { name: "Add or import" }));
  await userEvent.click(screen.getByRole("menuitem", { name: "Import Postman or .http…" }));
  const dialog = screen.getByRole("dialog", { name: "Import from Postman or .http" });
  expect(within(dialog).getByRole("button", { name: "Import" })).toBeDisabled();

  await userEvent.click(within(dialog).getByRole("button", { name: "Choose file…" }));
  const summary = await within(dialog).findByLabelText("Import summary");
  expect(summary).toHaveTextContent("1 requests in Orders");
  expect(summary).toHaveTextContent("Environments: Imported");
  expect(summary).toHaveTextContent("scripts are not run");
  await userEvent.selectOptions(
    within(dialog).getByRole("combobox", { name: "Import into" }),
    "/demo",
  );
  await userEvent.click(within(dialog).getByRole("button", { name: "Import" }));

  expect(await screen.findByText("Imported 1 request")).toBeInTheDocument();
  expect(await within(tree).findByText("List orders")).toBeInTheDocument();
});

it("a new collection that cannot be made is reported in the dialog", async () => {
  await openDemo();
  await userEvent.click(screen.getByRole("button", { name: "Add or import" }));
  await userEvent.click(screen.getByRole("menuitem", { name: "Import Postman or .http…" }));
  const dialog = screen.getByRole("dialog", { name: "Import from Postman or .http" });
  await userEvent.click(within(dialog).getByRole("button", { name: "Choose file…" }));
  await within(dialog).findByLabelText("Import summary");
  await userEvent.click(within(dialog).getByRole("button", { name: "Import" }));
  expect(await within(dialog).findByRole("alert")).toHaveTextContent("already holds a collection");
  await userEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
});

it("a folder exports as .http", async () => {
  const tree = await openDemo();
  fireEvent.contextMenu(within(tree).getByRole("button", { name: /Session/ }));
  await userEvent.click(screen.getByRole("menuitem", { name: "Export as .http…" }));
  expect(await screen.findByText("Exported 2 requests")).toBeInTheDocument();
});
