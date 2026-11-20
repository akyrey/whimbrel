import { useToasts } from "@roost/ui";
import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "../../App";
import { installMockBackend } from "../../test/mockBackend";
import { useCollections } from "../collections/collectionsStore";
import { useEnvironments } from "../environments/environmentsStore";
import { useTabs } from "../tabs/tabsStore";

beforeEach(() => {
  useToasts.setState({ toasts: [] });
  localStorage.clear();
  installMockBackend();
  useTabs.setState({ tabs: [], activeId: null });
  useCollections.setState({ views: {} });
  useEnvironments.setState({ entries: {}, active: {} });
});

afterEach(async () => {
  cleanup();
  await new Promise((resolve) => setTimeout(resolve, 0));
  clearMocks();
});

async function openTree() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open…" }));
  return screen.findByRole("region", { name: "Demo API" });
}

async function openRequest(tree: HTMLElement, name: RegExp) {
  await userEvent.click(within(tree).getByRole("button", { name }));
  await screen.findByRole("textbox", { name: "URL" });
}

async function preview() {
  await userEvent.click(screen.getByRole("button", { name: "Preview" }));
  const text = (await screen.findByLabelText("Request as sent")).textContent;
  await userEvent.click(
    within(screen.getByRole("dialog", { name: "Resolved request" })).getByRole("button", {
      name: "Close",
    }),
  );
  return text;
}

/** Opens "Headers…" on `target` and adds `name: value`. */
async function addInherited(target: HTMLElement, owner: string, name: string, value: string) {
  fireEvent.contextMenu(target);
  await userEvent.click(screen.getByRole("menuitem", { name: "Headers…" }));
  const dialog = await screen.findByRole("dialog", { name: `Headers of ${owner}` });
  await userEvent.click(within(dialog).getByRole("button", { name: "Add header" }));
  const index = within(dialog).getAllByRole("textbox", { name: /Header \d+ name/ }).length;
  await userEvent.type(
    within(dialog).getByRole("textbox", { name: `Header ${String(index)} name` }),
    name,
  );
  await userEvent.type(
    within(dialog).getByRole("textbox", { name: `Header ${String(index)} value` }),
    value,
  );
  await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
  await screen.findAllByText("Headers saved");
}

describe("inherited headers", () => {
  it("sends the collection's and folder's headers with every request under them", async () => {
    const tree = await openTree();
    await addInherited(
      within(tree).getByRole("heading", { name: "Demo API" }),
      "Demo API",
      "X-Tenant",
      "acme",
    );
    await addInherited(within(tree).getByRole("button", { name: /Users/ }), "Users", "X-Area", "u");

    await openRequest(tree, /List users/);
    const shown = await preview();

    expect(shown).toContain("X-Tenant: acme");
    expect(shown).toContain("X-Area: u");
    await openRequest(tree, /Ping/);
    const ping = await preview();
    expect(ping).toContain("X-Tenant: acme");
    expect(ping).not.toContain("X-Area");
  });

  it("lists inherited headers in the request, struck through when the request replaces one", async () => {
    const tree = await openTree();
    await addInherited(
      within(tree).getByRole("heading", { name: "Demo API" }),
      "Demo API",
      "Accept",
      "*/*",
    );
    await openRequest(tree, /Ping/);
    await userEvent.click(screen.getByRole("tab", { name: /Headers/ }));

    const inherited = await screen.findByRole("list", { name: "Inherited headers" });
    expect(within(inherited).getByText("Accept")).not.toHaveClass("line-through");
    await userEvent.click(screen.getByRole("button", { name: "Add header" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Header 1 name" }), "accept");

    expect(within(inherited).getByText("Accept")).toHaveClass("line-through");
    await userEvent.click(within(inherited).getByRole("button", { name: "Demo API" }));
    expect(await screen.findByRole("dialog", { name: "Headers of Demo API" })).toBeInTheDocument();
  });
});
