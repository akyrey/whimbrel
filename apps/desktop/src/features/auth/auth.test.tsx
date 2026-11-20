import { useToasts } from "@roost/ui";
import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
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

async function openPing() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open…" }));
  const tree = await screen.findByRole("region", { name: "Demo API" });
  await userEvent.click(within(tree).getByRole("button", { name: /Ping/ }));
  await screen.findByRole("textbox", { name: "URL" });
  return tree;
}

async function preview() {
  await userEvent.click(screen.getByRole("button", { name: "Preview" }));
  const text = await screen.findByLabelText("Request as sent");
  const content = text.textContent;
  await userEvent.click(
    within(screen.getByRole("dialog", { name: "Resolved request" })).getByRole("button", {
      name: "Close",
    }),
  );
  return content;
}

async function collectionAuth(tree: HTMLElement, pick: (dialog: HTMLElement) => Promise<void>) {
  fireEvent.contextMenu(within(tree).getByRole("heading", { name: "Demo API" }));
  await userEvent.click(screen.getByRole("menuitem", { name: "Auth…" }));
  const dialog = await screen.findByRole("dialog", { name: "Auth of Demo API" });
  await pick(dialog);
  await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
  await screen.findAllByText("Auth saved");
}

describe("auth", () => {
  it("applies the request's own bearer token, masked in the preview", async () => {
    await openPing();
    await userEvent.click(screen.getByText("Auth"));
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Auth type" }), "bearer");
    await userEvent.type(screen.getByRole("textbox", { name: "Token" }), "abc");

    expect(await preview()).toContain("Authorization: Bearer ••••••");
    await userEvent.click(screen.getByRole("button", { name: "Send" }));
    expect(await screen.findByLabelText("Response body")).toHaveTextContent("Bearer abc");
  });

  it("inherits the collection's auth, which the collection cannot itself inherit", async () => {
    const tree = await openPing();
    await collectionAuth(tree, async (dialog) => {
      const type = within(dialog).getByRole("combobox", { name: "Auth type" });
      expect(within(type).queryByRole("option", { name: "Inherit" })).toBeNull();
      await userEvent.selectOptions(type, "basic");
      await userEvent.type(within(dialog).getByRole("textbox", { name: "Username" }), "ada");
      await userEvent.type(within(dialog).getByRole("textbox", { name: "Password" }), "pw");
    });

    expect(await preview()).toContain("Authorization: Basic ••••••");
  });

  it("gets an OAuth 2 token, sends with it, and forgets it", async () => {
    const tree = await openPing();
    await collectionAuth(tree, async (dialog) => {
      await userEvent.selectOptions(
        within(dialog).getByRole("combobox", { name: "Auth type" }),
        "oauth2",
      );
      await userEvent.type(
        within(dialog).getByRole("textbox", { name: "Token URL" }),
        "https://idp/token",
      );
      await userEvent.type(within(dialog).getByRole("textbox", { name: "Client ID" }), "app");
    });
    await userEvent.click(screen.getByText("Auth"));
    const panel = await screen.findByRole("group", { name: "OAuth 2 token" });
    expect(panel).toHaveTextContent("No token yet.");

    await userEvent.click(screen.getByRole("button", { name: "Send" }));
    expect(await screen.findByText(/needs an OAuth 2 token/)).toBeInTheDocument();
    await userEvent.click(within(panel).getByRole("button", { name: "Get token" }));
    await waitFor(() => {
      expect(screen.getByRole("group", { name: "OAuth 2 token" })).toHaveTextContent(
        "Using “Token 1”",
      );
    });
    await userEvent.click(screen.getByRole("button", { name: "Send" }));
    expect(await screen.findByLabelText("Response body")).toHaveTextContent("Bearer access-1");
    await userEvent.click(screen.getByRole("button", { name: "Forget token" }));
    await waitFor(() => {
      expect(screen.getByRole("group", { name: "OAuth 2 token" })).toHaveTextContent(
        "No token yet.",
      );
    });
  });

  it("names tokens, keeps several, and switches, renames and deletes them", async () => {
    const tree = await openPing();
    await collectionAuth(tree, async (dialog) => {
      await userEvent.selectOptions(
        within(dialog).getByRole("combobox", { name: "Auth type" }),
        "oauth2",
      );
      await userEvent.type(
        within(dialog).getByRole("textbox", { name: "Token URL" }),
        "https://idp/token",
      );
    });
    await userEvent.click(screen.getByText("Auth"));
    const panel = () => screen.getByRole("group", { name: "OAuth 2 token" });
    await screen.findByRole("group", { name: "OAuth 2 token" });

    await userEvent.type(within(panel()).getByRole("textbox", { name: "New token name" }), "Alice");
    await userEvent.click(within(panel()).getByRole("button", { name: "Get token" }));
    await waitFor(() => {
      expect(panel()).toHaveTextContent("Using “Alice”");
    });
    expect(within(panel()).getByRole("textbox", { name: "New token name" })).toHaveValue("");
    await userEvent.click(within(panel()).getByRole("button", { name: "Get a new token" }));
    await waitFor(() => {
      expect(panel()).toHaveTextContent("Using “Token 1”");
    });
    await userEvent.click(screen.getByRole("button", { name: "Send" }));
    expect(await screen.findByLabelText("Response body")).toHaveTextContent("Bearer access-2");

    await userEvent.click(within(panel()).getByRole("button", { name: "Manage tokens (2)" }));
    const dialog = await screen.findByRole("dialog", { name: "OAuth 2 tokens" });
    await userEvent.click(within(dialog).getByRole("radio", { name: "Use Alice" }));
    await waitFor(() => {
      expect(within(dialog).getByRole("radio", { name: "Use Alice" })).toBeChecked();
    });
    const second = within(dialog).getByRole("textbox", { name: "Name of Token 1" });
    await userEvent.clear(second);
    await userEvent.type(second, "Bob{Enter}");
    expect(await within(dialog).findByRole("radio", { name: "Use Bob" })).not.toBeChecked();
    await userEvent.click(within(dialog).getByRole("button", { name: "Close" }));
    expect(panel()).toHaveTextContent("Using “Alice”");
    await userEvent.click(screen.getByRole("button", { name: "Send" }));
    await waitFor(() => {
      expect(screen.getByLabelText("Response body")).toHaveTextContent("Bearer access-1");
    });

    await userEvent.click(within(panel()).getByRole("button", { name: "Manage tokens (2)" }));
    const again = await screen.findByRole("dialog", { name: "OAuth 2 tokens" });
    await userEvent.click(within(again).getByRole("button", { name: "Delete Alice" }));
    await waitFor(() => {
      expect(within(again).queryByRole("radio", { name: "Use Alice" })).toBeNull();
    });
    await userEvent.click(within(again).getByRole("button", { name: "Close" }));
    expect(panel()).toHaveTextContent("No token in use");
  });

  it("refuses an empty token name when renaming", async () => {
    const tree = await openPing();
    await collectionAuth(tree, async (dialog) => {
      await userEvent.selectOptions(
        within(dialog).getByRole("combobox", { name: "Auth type" }),
        "oauth2",
      );
    });
    await userEvent.click(screen.getByText("Auth"));
    const panel = await screen.findByRole("group", { name: "OAuth 2 token" });
    await userEvent.click(within(panel).getByRole("button", { name: "Get token" }));
    await userEvent.click(await within(panel).findByRole("button", { name: "Manage tokens (1)" }));
    const dialog = await screen.findByRole("dialog", { name: "OAuth 2 tokens" });
    const name = within(dialog).getByRole("textbox", { name: "Name of Token 1" });

    await userEvent.clear(name);
    await userEvent.tab();

    expect(await screen.findByText("A token needs a name")).toBeInTheDocument();
    expect(name).toHaveValue("Token 1");
  });

  it("shows the fields of each API key place and OAuth grant", async () => {
    await openPing();
    await userEvent.click(screen.getByText("Auth"));
    const type = screen.getByRole("combobox", { name: "Auth type" });
    await userEvent.selectOptions(type, "api_key");
    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "key");
    await userEvent.type(screen.getByRole("textbox", { name: "Value" }), "v");
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Sent in" }), "query");
    expect(await preview()).toContain("?key=••••••");

    await userEvent.selectOptions(type, "oauth2");
    const grant = screen.getByRole("combobox", { name: "Grant" });
    await userEvent.selectOptions(grant, "authorization_code");
    expect(screen.getByRole("textbox", { name: "Authorization URL" })).toBeInTheDocument();
    await userEvent.selectOptions(grant, "device_code");
    expect(screen.getByRole("textbox", { name: "Device URL" })).toBeInTheDocument();
    await userEvent.selectOptions(type, "none");
    expect(useTabs.getState().tabs[0]?.draft.auth).toEqual({ type: "none" });
  });

  it("sets a folder's auth from its menu, which the requests inside inherit", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "Open…" }));
    const tree = await screen.findByRole("region", { name: "Demo API" });
    fireEvent.contextMenu(within(tree).getByRole("button", { name: /Users/ }));
    await userEvent.click(screen.getByRole("menuitem", { name: "Auth…" }));
    const dialog = await screen.findByRole("dialog", { name: "Auth of Users" });
    const type = within(dialog).getByRole("combobox", { name: "Auth type" });
    expect(within(type).getByRole("option", { name: "Inherit" })).toBeInTheDocument();
    await userEvent.selectOptions(type, "bearer");
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Token" }), "folder-token");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await screen.findAllByText("Auth saved");

    await userEvent.click(await within(tree).findByRole("button", { name: /List users/ }));
    await screen.findByRole("textbox", { name: "URL" });
    expect(await preview()).toContain("Authorization: Bearer ••••••");
  });

  it("links an inheriting request to the folders and collection it inherits from", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "Open…" }));
    const tree = await screen.findByRole("region", { name: "Demo API" });
    await userEvent.click(await within(tree).findByRole("button", { name: /List users/ }));
    await userEvent.click(await screen.findByText("Auth"));

    const from = screen.getByRole("group", { name: "Inherited auth" });
    expect(within(from).getByRole("button", { name: "Users" })).toBeInTheDocument();
    await userEvent.click(within(from).getByRole("button", { name: "Demo API" }));
    const dialog = await screen.findByRole("dialog", { name: "Auth of Demo API" });
    await userEvent.selectOptions(
      within(dialog).getByRole("combobox", { name: "Auth type" }),
      "bearer",
    );
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Token" }), "t");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await screen.findAllByText("Auth saved");
    expect(await preview()).toContain("Authorization: Bearer ••••••");
  });

  it("shows no inherited auth links when the request has its own", async () => {
    await openPing();
    await userEvent.click(screen.getByText("Auth"));
    expect(screen.getByRole("group", { name: "Inherited auth" })).toBeInTheDocument();
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Auth type" }), "none");
    expect(screen.queryByRole("group", { name: "Inherited auth" })).toBeNull();
  });

  it("saves the collection's TLS files", async () => {
    const tree = await openPing();
    fireEvent.contextMenu(within(tree).getByRole("heading", { name: "Demo API" }));
    await userEvent.click(screen.getByRole("menuitem", { name: "TLS…" }));
    const dialog = await screen.findByRole("dialog", { name: "TLS" });
    await userEvent.type(
      await within(dialog).findByRole("textbox", { name: "Extra trusted CA" }),
      "certs/ca.pem",
    );
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));

    expect(await screen.findByText("TLS settings saved")).toBeInTheDocument();
  });
});
