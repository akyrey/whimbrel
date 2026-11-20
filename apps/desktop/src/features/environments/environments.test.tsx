import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "../../App";
import { installMockBackend } from "../../test/mockBackend";
import { useCollections } from "../collections/collectionsStore";
import { useTabs } from "../tabs/tabsStore";
import { useEnvironments } from "./environmentsStore";

beforeEach(() => {
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

async function openPing() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open…" }));
  const tree = await screen.findByRole("region", { name: "Demo API" });
  await userEvent.click(within(tree).getByRole("button", { name: /Ping/ }));
  await screen.findByRole("textbox", { name: "URL" });
  return tree;
}

async function setUrl(url: string) {
  const input = screen.getByRole("textbox", { name: "URL" });
  await userEvent.clear(input);
  await userEvent.type(input, url.replaceAll("{", "{{"));
}

async function pickEnvironment(name: string) {
  const select = await screen.findByRole("combobox", { name: "Environment" });
  await waitFor(() => {
    expect(within(select).getByRole("option", { name: new RegExp(name) })).toBeInTheDocument();
  });
  await userEvent.selectOptions(
    select,
    within(select).getByRole("option", { name: new RegExp(name) }),
  );
}

async function preview() {
  await userEvent.click(screen.getByRole("button", { name: "Preview" }));
  const dialog = await screen.findByRole("dialog", { name: "Resolved request" });
  return dialog;
}

const closeDialog = () => userEvent.click(screen.getByRole("button", { name: "Close" }));

describe("variables and environments", () => {
  it("resolves from the active environment, and names what is not defined", async () => {
    await openPing();
    await setUrl("{{base}}/echo");

    expect(await within(await preview()).findByRole("alert")).toHaveTextContent(
      "Not defined: base",
    );
    await closeDialog();
    await pickEnvironment("Local");
    expect(await within(await preview()).findByLabelText("Request as sent")).toHaveTextContent(
      "GET http://localhost/echo",
    );
    await closeDialog();
    await userEvent.click(screen.getByRole("button", { name: "Send" }));
    expect(await screen.findByText("200 OK")).toBeInTheDocument();
  });

  it("makes a saved environment variable known to open tabs at once", async () => {
    await openPing();
    await pickEnvironment("Local");
    await setUrl("{{base}}/{{fresh}}");
    const used = screen.getByRole("list", { name: "Variables used" });
    expect(await within(used).findByLabelText("fresh is not defined")).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Edit environments" }));
    const editor = screen.getByRole("dialog", { name: "Environments" });
    await userEvent.click(within(editor).getByRole("button", { name: /Local/ }));
    await userEvent.click(
      (await within(editor).findAllByRole("button", { name: "Add" }))[0] as HTMLElement,
    );
    const names = within(editor).getAllByRole("textbox", { name: /^Variables \d+ name$/ });
    const values = within(editor).getAllByRole("textbox", { name: /^Variables \d+ value$/ });
    await userEvent.type(names.at(-1) as HTMLElement, "fresh");
    await userEvent.type(values.at(-1) as HTMLElement, "new");
    await userEvent.click(within(editor).getByRole("button", { name: "Save" }));
    expect((await screen.findAllByText("Saved")).length).toBeGreaterThan(0);

    expect(
      await within(used).findByLabelText("fresh = new (from the environment)"),
    ).toBeInTheDocument();
  });

  it("shows the active environment beside the URL, protected ones in rose", async () => {
    await openPing();
    const chip = () => screen.getByRole("button", { name: /^Environment:/ });
    expect(chip()).toHaveTextContent("No environment");

    await pickEnvironment("Local");
    expect(chip()).toHaveTextContent("Local");
    expect(chip()).not.toHaveClass("text-rose-300");

    await pickEnvironment("Production");
    expect(chip()).toHaveTextContent("🔒 Production");
    expect(chip()).toHaveClass("text-rose-300");

    await userEvent.click(chip());
    expect(screen.getByRole("combobox", { name: "Environment" })).toHaveFocus();
  });

  it("stores a secret, masks it, asks before a protected send, and refuses to save it", async () => {
    await openPing();
    await setUrl("{{base}}/echo");
    await userEvent.click(screen.getByRole("tab", { name: /^Headers/ }));
    await userEvent.click(screen.getByRole("button", { name: "Add header" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Header 1 name" }), "X-Api-Key");
    await userEvent.type(screen.getByRole("textbox", { name: "Header 1 value" }), "{{{{apiKey}}");
    await pickEnvironment("Production");

    expect(await within(await preview()).findByRole("alert")).toHaveTextContent(
      "No value in the keychain for: apiKey",
    );
    await closeDialog();
    await userEvent.click(screen.getByRole("button", { name: "Edit environments" }));
    const editor = screen.getByRole("dialog", { name: "Environments" });
    await userEvent.click(within(editor).getByRole("button", { name: /Production/ }));
    await userEvent.type(await within(editor).findByLabelText("Value of apiKey"), "sk-live-9999");
    await userEvent.click(within(editor).getByRole("button", { name: "Set" }));
    expect(await within(editor).findByText("set")).toBeInTheDocument();
    await userEvent.click(
      within(editor).getAllByRole("button", { name: "Close" }).at(-1) as HTMLElement,
    );

    const resolved = await within(await preview()).findByLabelText("Request as sent");
    expect(resolved).toHaveTextContent("X-Api-Key: ••••••");
    expect(resolved).not.toHaveTextContent("sk-live-9999");
    await closeDialog();
    await userEvent.click(screen.getByRole("button", { name: "Send" }));
    await userEvent.click(
      within(screen.getByRole("dialog", { name: /protected environment/ })).getByRole("button", {
        name: "Send",
      }),
    );
    expect(await screen.findByText("200 OK")).toBeInTheDocument();

    await userEvent.clear(screen.getByRole("textbox", { name: "Header 1 value" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Header 1 value" }), "sk-live-9999");
    await userEvent.keyboard("{Control>}s{/Control}");
    expect(await screen.findByText(/holds the value of the secret apiKey/)).toBeInTheDocument();
  });

  it("creates, edits and deletes an environment, and saves collection variables", async () => {
    await openPing();
    await userEvent.click(screen.getByRole("button", { name: "Edit environments" }));
    const editor = screen.getByRole("dialog", { name: "Environments" });

    await userEvent.click(within(editor).getByRole("button", { name: "New environment" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "Staging{Enter}");
    await userEvent.click(await within(editor).findByRole("checkbox"));
    await userEvent.click(within(editor).getAllByRole("button", { name: "Add" })[0] as HTMLElement);
    await userEvent.type(within(editor).getByRole("textbox", { name: "Variables 1 name" }), "base");
    await userEvent.type(
      within(editor).getByRole("textbox", { name: "Variables 1 value" }),
      "https://staging",
    );
    await userEvent.click(within(editor).getByRole("button", { name: "Add secret" }));
    await userEvent.type(within(editor).getByRole("textbox", { name: "Secret 1 name" }), "token");
    await userEvent.click(within(editor).getByRole("button", { name: "Save" }));
    expect(await within(editor).findByRole("button", { name: /Staging 🔒/ })).toBeInTheDocument();
    expect(await within(editor).findByText("not set")).toBeInTheDocument();

    await userEvent.click(within(editor).getByRole("button", { name: "Delete" }));
    await userEvent.click(
      within(screen.getByRole("dialog", { name: "Delete environment?" })).getByRole("button", {
        name: "Delete",
      }),
    );
    await waitFor(() => {
      expect(within(editor).queryByRole("button", { name: /Staging/ })).toBeNull();
    });
    await userEvent.click(within(editor).getByRole("button", { name: "Add" }));
    await userEvent.type(
      within(editor).getByRole("textbox", { name: "Collection variables 1 name" }),
      "version",
    );
    await userEvent.click(within(editor).getByRole("button", { name: "Save" }));
    expect((await screen.findAllByText("Saved")).length).toBeGreaterThan(0);
  });
});

describe("examples", () => {
  it("saves edits as an example, sends with it, applies, updates and deletes it", async () => {
    const tree = await openPing();
    await pickEnvironment("Local");
    await setUrl("http://localhost/echo?page=2");

    await userEvent.click(screen.getByRole("button", { name: "Save as example…" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Example name" }), "Page 2{Enter}");

    expect(await within(tree).findByRole("button", { name: /Page 2/ })).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "URL" })).toHaveValue("http://localhost/echo");
    expect(screen.getByRole("combobox", { name: "Example" })).toHaveDisplayValue("Page 2");
    expect(await within(await preview()).findByLabelText("Request as sent")).toHaveTextContent(
      "echo?page=2",
    );
    await closeDialog();

    await userEvent.click(screen.getByRole("button", { name: "Apply to editor" }));
    await waitFor(() => {
      expect(screen.getByRole("textbox", { name: "URL" })).toHaveValue(
        "http://localhost/echo?page=2",
      );
    });
    await userEvent.click(screen.getByRole("button", { name: "Update example" }));
    await userEvent.click(screen.getByRole("button", { name: "Delete example" }));
    await userEvent.click(
      within(screen.getByRole("dialog", { name: "Delete example?" })).getByRole("button", {
        name: "Delete",
      }),
    );
    expect(screen.getByRole("combobox", { name: "Example" })).toHaveDisplayValue("No example");
  });

  it("opens a request with an example picked from the tree, and refuses a path change", async () => {
    const tree = await openPing();
    await setUrl("http://localhost/echo?x=1");
    await userEvent.click(screen.getByRole("button", { name: "Save as example…" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Example name" }), "X{Enter}");
    await within(tree).findByRole("button", { name: /↳ X/ });
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Example" }), "No example");

    await userEvent.click(within(tree).getByRole("button", { name: /↳ X/ }));
    expect(screen.getByRole("combobox", { name: "Example" })).toHaveDisplayValue("X");

    await setUrl("http://localhost/other");
    await userEvent.click(screen.getByRole("button", { name: "Save as example…" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Example name" }), "Bad{Enter}");
    expect(await screen.findByText(/keeps the request's method and path/)).toBeInTheDocument();
  });
});
