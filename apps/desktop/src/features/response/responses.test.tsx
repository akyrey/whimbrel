import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "../../App";
import { installMockBackend } from "../../test/mockBackend";
import { useCollections } from "../collections/collectionsStore";
import { useEnvironments } from "../environments/environmentsStore";
import { useTabs } from "../tabs/tabsStore";

const writeText = vi.fn<(text: string) => Promise<void>>(() => Promise.resolve());

beforeEach(() => {
  localStorage.clear();
  installMockBackend();
  useTabs.setState({ tabs: [], activeId: null });
  useCollections.setState({ views: {} });
  useEnvironments.setState({ entries: {}, active: {} });
  Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
  writeText.mockClear();
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(400);
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(800);
});

afterEach(async () => {
  cleanup();
  vi.restoreAllMocks();
  await new Promise((resolve) => setTimeout(resolve, 0));
  clearMocks();
});

async function send(url: string) {
  if (!screen.queryByRole("textbox", { name: "URL" })) {
    await userEvent.click(screen.getByRole("button", { name: "New request" }));
  }
  const input = screen.getByRole("textbox", { name: "URL" });
  await userEvent.clear(input);
  await userEvent.type(input, url);
  await userEvent.click(screen.getByRole("button", { name: "Send" }));
  return url.includes("unreachable")
    ? screen.findByRole("alert")
    : screen.findByRole("region", { name: "Response" });
}

describe("response views", () => {
  it("shows a JSON array as a table, raw text, and the cookies it set", async () => {
    render(<App />);
    const panel = await send("http://localhost/users");

    await userEvent.click(await within(panel).findByRole("radio", { name: "Table" }));
    const table = within(panel).getByLabelText("Body as a table");
    expect(within(table).getByRole("columnheader", { name: "admin" })).toBeInTheDocument();
    expect(within(table).getByText("grace")).toBeInTheDocument();
    await userEvent.click(within(panel).getByRole("radio", { name: "Raw" }));
    expect(await within(panel).findByLabelText("Response body")).toHaveTextContent('"name":"ada"');
    await userEvent.click(within(panel).getByRole("tab", { name: "Cookies (1)" }));
    expect(within(panel).getByText("session")).toBeInTheDocument();
  });

  it("previews images and shows binary bodies as hex", async () => {
    render(<App />);
    const image = await send("http://localhost/image");
    expect(
      within(image).getByRole("img", { name: "The response body" }).getAttribute("src"),
    ).toMatch(/^data:image\/png;base64,/);

    const binary = await send("http://localhost/binary");
    expect(within(binary).getByLabelText("Body as hex")).toHaveTextContent("50 4b 03 04");
  });

  it("says when only the start is shown, and saves the whole body", async () => {
    render(<App />);
    const panel = await send("http://localhost/spilled");

    expect(within(panel).getByRole("note")).toHaveTextContent(
      "Showing the first part of 50.0 MB. Save to file for all of it.",
    );
    await userEvent.click(within(panel).getByRole("button", { name: "Save to file" }));
    expect(await screen.findByText("Saved to /tmp/saved-body")).toBeInTheDocument();
  });
});

describe("request settings", () => {
  it("edits the timeout, redirects and certificate check of the request", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New request" }));
    await userEvent.click(screen.getByText("Settings"));
    await userEvent.type(
      screen.getByRole("spinbutton", { name: "Timeout in milliseconds" }),
      "500",
    );
    await userEvent.click(screen.getByRole("checkbox", { name: "Follow redirects" }));
    await userEvent.click(screen.getByRole("checkbox", { name: "Verify certificates" }));

    expect(screen.getByText(/Any certificate is accepted/)).toBeInTheDocument();
    expect(useTabs.getState().tabs[0]?.draft.settings).toEqual({
      timeoutMs: 500,
      followRedirects: false,
      verifyTls: false,
    });
    await userEvent.clear(screen.getByRole("spinbutton", { name: "Timeout in milliseconds" }));
    expect(useTabs.getState().tabs[0]?.draft.settings.timeoutMs).toBeNull();
  });
});

describe("history", () => {
  it("lists what was sent, searches, reopens an entry with its response, and clears", async () => {
    render(<App />);
    await send("http://localhost/users");
    await send("http://unreachable.test/");

    await userEvent.click(screen.getByRole("tab", { name: "History" }));
    const history = screen.getByRole("navigation", { name: "History" });
    await waitFor(() => {
      expect(within(history).getAllByRole("button", { name: /http/ })).toHaveLength(2);
    });
    await userEvent.type(
      within(history).getByRole("textbox", { name: "Search the history" }),
      "users",
    );
    await waitFor(() => {
      expect(within(history).getAllByRole("button", { name: /http/ })).toHaveLength(1);
    });
    await userEvent.click(within(history).getByRole("button", { name: /users/ }));
    expect(useTabs.getState().tabs).toHaveLength(2);
    expect(await screen.findByText("200 OK")).toBeInTheDocument();

    await userEvent.click(within(history).getByRole("button", { name: "Clear" }));
    await userEvent.click(
      within(screen.getByRole("dialog", { name: "Clear the history?" })).getByRole("button", {
        name: "Clear",
      }),
    );
    expect(await within(history).findByText("Nothing sent yet.")).toBeInTheDocument();
  });
});

describe("cURL", () => {
  it("imports a pasted command and copies a request back as cURL", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "Import cURL…" }));
    await userEvent.type(
      screen.getByRole("textbox", { name: "cURL command" }),
      "curl -X PUT 'http://localhost/echo' -H 'X-Api: 1' --compressed",
    );
    await userEvent.click(screen.getByRole("button", { name: "Import" }));

    expect(await screen.findByRole("textbox", { name: "URL" })).toHaveValue(
      "http://localhost/echo",
    );
    expect(screen.getByRole("combobox", { name: "Method" })).toHaveValue("PUT");
    expect(screen.getByText("Not imported: --compressed")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Copy cURL" }));
    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith(
        "curl -X PUT 'http://localhost/echo' \\\n  -H 'X-Api: 1'",
      );
    });
  });

  it("explains a command it cannot read", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "Import cURL…" }));
    await userEvent.type(screen.getByRole("textbox", { name: "cURL command" }), "wget http://x");
    await userEvent.click(screen.getByRole("button", { name: "Import" }));
    expect(await screen.findByText("This is not a cURL command")).toBeInTheDocument();
  });
});
