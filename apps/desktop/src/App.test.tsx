import { clearMocks } from "@tauri-apps/api/mocks";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "./App";
import { useCollections } from "./features/collections/collectionsStore";
import { useTabs } from "./features/tabs/tabsStore";
import { externalEdit, installMockBackend } from "./test/mockBackend";

beforeEach(() => {
  localStorage.clear();
  installMockBackend();
  useTabs.setState({ tabs: [], activeId: null });
  useCollections.setState({ views: {} });
});

afterEach(async () => {
  // Unmount first, and let the app stop listening (a pending promise) before the mocked IPC
  // it listens through is removed.
  cleanup();
  await new Promise((resolve) => setTimeout(resolve, 0));
  clearMocks();
});

async function typeUrl(url: string) {
  const input = screen.getByRole("textbox", { name: "URL" });
  await userEvent.clear(input);
  await userEvent.type(input, url);
}

async function openDemo() {
  await userEvent.click(screen.getByRole("button", { name: "Open…" }));
  return screen.findByRole("region", { name: "Demo API" });
}

async function openPing() {
  const tree = await openDemo();
  await userEvent.click(within(tree).getByRole("button", { name: /Ping/ }));
  await screen.findByRole("textbox", { name: "URL" });
}

const tabNamed = (name: string) => screen.getByRole("tab", { name: new RegExp(name) });

describe("a scratch request", () => {
  it("sends with a header and shows the pretty-printed response", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New request" }));
    await userEvent.clear(screen.getByRole("combobox", { name: "Method" }));
    await userEvent.type(screen.getByRole("combobox", { name: "Method" }), "post");
    await typeUrl("http://localhost/echo");
    await userEvent.click(screen.getByRole("tab", { name: /^Headers/ }));
    await userEvent.click(screen.getByRole("button", { name: "Add header" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Header 1 name" }), "X-Api");
    await userEvent.click(screen.getByRole("button", { name: "Send" }));

    const panel = await screen.findByRole("region", { name: "Response" });
    expect(within(panel).getByText("200 OK")).toBeInTheDocument();
    const body = await within(panel).findByLabelText("Response body");
    expect(body).toHaveTextContent('"method": "POST"');
    await userEvent.click(within(panel).getByRole("tab", { name: "Headers (2)" }));
    expect(within(panel).getByText("x-mock")).toBeInTheDocument();
  });

  it("explains failures, cancels, and asks a collection before saving", async () => {
    render(<App />);
    fireEvent.keyDown(window, { key: "t", ctrlKey: true });
    await typeUrl("http://unreachable.test/");
    await userEvent.click(screen.getByRole("button", { name: "Send" }));
    expect(await screen.findByText("Could not connect (connection failed)")).toBeInTheDocument();

    await typeUrl("http://localhost/slow");
    await userEvent.click(screen.getByRole("button", { name: "Send" }));
    await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));
    expect(await screen.findByText("Cancelled")).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "s", ctrlKey: true });
    expect(
      await screen.findByText("Open or create a collection to save this request in."),
    ).toBeInTheDocument();
  });

  it("is saved into the open collection", async () => {
    render(<App />);
    const tree = await openDemo();
    fireEvent.keyDown(window, { key: "t", ctrlKey: true });
    await typeUrl("http://localhost/new");
    fireEvent.keyDown(window, { key: "s", ctrlKey: true });

    expect(await within(tree).findByRole("button", { name: /New request/ })).toBeInTheDocument();
    await waitFor(() => {
      expect(tabNamed("New request")).not.toHaveTextContent("•");
    });
  });
});

describe("collections", () => {
  it("opens a request, marks edits unsaved, and saves them", async () => {
    render(<App />);
    await openPing();

    await typeUrl("http://localhost/echo?x=1");
    expect(tabNamed("Ping")).toHaveTextContent("•");
    fireEvent.keyDown(window, { key: "s", ctrlKey: true });

    await waitFor(() => {
      expect(tabNamed("Ping")).not.toHaveTextContent("•");
    });
  });

  it("follows a change on disk when saved, and asks when not", async () => {
    render(<App />);
    await openPing();

    await act(() =>
      externalEdit("ping.req.toml", (s) => ({
        ...s,
        request: { ...s.request, url: "http://disk/1" },
      })),
    );
    await waitFor(() => {
      expect(screen.getByRole("textbox", { name: "URL" })).toHaveValue("http://disk/1");
    });

    await typeUrl("http://mine");
    await act(() =>
      externalEdit("ping.req.toml", (s) => ({
        ...s,
        request: { ...s.request, url: "http://disk/2" },
      })),
    );
    const bar = await screen.findByText("This request changed on disk while you were editing it.");
    expect(screen.getByRole("textbox", { name: "URL" })).toHaveValue("http://mine");
    await userEvent.click(
      within(bar.parentElement as HTMLElement).getByRole("button", { name: "Take theirs" }),
    );
    expect(screen.getByRole("textbox", { name: "URL" })).toHaveValue("http://disk/2");

    await act(() => externalEdit("ping.req.toml", null));
    expect(
      await screen.findByText(/The file of this request is gone from disk/),
    ).toBeInTheDocument();
  });

  it("creates, renames and deletes requests from the menus", async () => {
    render(<App />);
    const tree = await openDemo();

    fireEvent.contextMenu(within(tree).getByRole("heading", { name: "Demo API" }));
    await userEvent.click(screen.getByRole("menuitem", { name: "New request" }));
    const name = screen.getByRole("textbox", { name: "Name" });
    await userEvent.clear(name);
    await userEvent.type(name, "Health{Enter}");
    expect(await within(tree).findByRole("button", { name: /Health/ })).toBeInTheDocument();
    expect(tabNamed("Health")).toBeInTheDocument();

    fireEvent.contextMenu(within(tree).getByRole("button", { name: /Health/ }));
    await userEvent.click(screen.getByRole("menuitem", { name: "Rename" }));
    await userEvent.clear(screen.getByRole("textbox", { name: "Name" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "Health check{Enter}");
    expect(await within(tree).findByRole("button", { name: /Health check/ })).toBeInTheDocument();
    expect(tabNamed("Health check")).toBeInTheDocument();

    fireEvent.contextMenu(within(tree).getByRole("button", { name: /Health check/ }));
    await userEvent.click(screen.getByRole("menuitem", { name: "Delete" }));
    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    await waitFor(() => {
      expect(within(tree).queryByRole("button", { name: /Health check/ })).toBeNull();
    });
    expect(screen.queryByRole("tab", { name: /Health check/ })).toBeNull();
  });

  it("duplicates, adds folders, and reopens what was open after a restart", async () => {
    const { unmount } = render(<App />);
    const tree = await openDemo();
    fireEvent.contextMenu(within(tree).getByRole("button", { name: /Ping/ }));
    await userEvent.click(screen.getByRole("menuitem", { name: "Duplicate" }));
    expect(await within(tree).findByRole("button", { name: /Ping copy/ })).toBeInTheDocument();
    fireEvent.contextMenu(within(tree).getByRole("button", { name: "Users" }));
    await userEvent.click(screen.getByRole("menuitem", { name: "New folder" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "Admin{Enter}");
    expect(await within(tree).findByRole("button", { name: "Admin" })).toBeInTheDocument();
    unmount();

    useTabs.setState({ tabs: [], activeId: null });
    useCollections.setState({ views: {} });
    render(<App />);

    expect(await screen.findByRole("region", { name: "Demo API" })).toBeInTheDocument();
    expect(await screen.findByRole("tab", { name: /Ping copy/ })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });

  it("asks before closing an unsaved tab", async () => {
    render(<App />);
    await openPing();
    await typeUrl("http://changed");

    await userEvent.click(screen.getByRole("button", { name: "Close Ping" }));
    await userEvent.click(screen.getByRole("button", { name: "Close" }));

    expect(screen.queryByRole("tab", { name: /Ping/ })).toBeNull();
  });
});

describe("keyboard", () => {
  it("opens the palette and the shortcuts, and focuses the URL", async () => {
    render(<App />);
    fireEvent.keyDown(window, { key: "t", ctrlKey: true });
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    const palette = screen.getByRole("dialog", { name: "Command palette" });
    await userEvent.type(within(palette).getByRole("combobox"), "shortcuts{Enter}");
    expect(screen.getByRole("dialog", { name: "Keyboard shortcuts" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Close" }));

    fireEvent.keyDown(window, { key: "l", ctrlKey: true });
    expect(screen.getByRole("textbox", { name: "URL" })).toHaveFocus();
    fireEvent.keyDown(window, { key: "w", ctrlKey: true });
    expect(screen.queryByRole("textbox", { name: "URL" })).toBeNull();
  });
});
