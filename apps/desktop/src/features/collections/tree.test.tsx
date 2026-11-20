import { useToasts } from "@roost/ui";
import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "../../App";
import { installMockBackend } from "../../test/mockBackend";
import { useTabs } from "../tabs/tabsStore";
import { useCollections } from "./collectionsStore";

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

async function menu(tree: HTMLElement, name: RegExp, item: string) {
  fireEvent.contextMenu(within(tree).getByRole("button", { name }));
  await userEvent.click(screen.getByRole("menuitem", { name: item }));
}

/** The requests in a folder, in the order shown. */
function requestsIn(tree: HTMLElement, folder: RegExp): string[] {
  const li = within(tree).getByRole("button", { name: folder }).closest("li") as HTMLElement;
  return within(li)
    .getAllByRole("button")
    .map((b) => b.textContent)
    .filter((text) => /^(GET|POST)/.test(text));
}

it("folders are renamed and requests move up and down", async () => {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open…" }));
  const tree = await screen.findByRole("region", { name: "Demo API" });

  await menu(tree, /Session/, "Rename");
  const name = screen.getByRole("textbox", { name: "Name" });
  await userEvent.clear(name);
  await userEvent.type(name, "Auth flow{Enter}");
  expect(await within(tree).findByRole("button", { name: /Auth flow/ })).toBeInTheDocument();
  expect(requestsIn(tree, /Auth flow/)).toEqual(["POSTGet a token", "GETMe"]);

  fireEvent.contextMenu(within(tree).getByRole("button", { name: /Get a token/ }));
  expect(screen.queryByRole("menuitem", { name: "Move up" })).toBeNull();
  await userEvent.click(screen.getByRole("menuitem", { name: "Move down" }));
  await waitFor(() => {
    expect(requestsIn(tree, /Auth flow/)).toEqual(["GETMe", "POSTGet a token"]);
  });

  await menu(tree, /Get a token/, "Move up");
  await waitFor(() => {
    expect(requestsIn(tree, /Auth flow/)).toEqual(["POSTGet a token", "GETMe"]);
  });
});

it("filters the tree by name, keeps the folders above matches, and Escape clears it", async () => {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open…" }));
  const tree = await screen.findByRole("region", { name: "Demo API" });
  const filter = screen.getByRole("searchbox", { name: "Filter requests" });

  await userEvent.type(filter, "TOKEN");
  expect(within(tree).getByRole("button", { name: /Session/ })).toHaveAttribute(
    "aria-expanded",
    "true",
  );
  expect(within(tree).getByRole("button", { name: /Get a token/ })).toBeInTheDocument();
  expect(within(tree).queryByRole("button", { name: /Ping/ })).toBeNull();
  expect(within(tree).queryByRole("button", { name: /Users/ })).toBeNull();

  await userEvent.clear(filter);
  await userEvent.type(filter, "nothing-here");
  expect(within(tree).getByText("No matches.")).toBeInTheDocument();

  await userEvent.type(filter, "{Escape}");
  expect(filter).toHaveValue("");
  expect(within(tree).getByRole("button", { name: /Ping/ })).toBeInTheDocument();
  expect(within(tree).getByRole("button", { name: /Forged/ })).toBeInTheDocument();
});

describe("drag and drop", () => {
  /** jsdom has no DataTransfer; the tree only sets data and effects on it. */
  const transfer = () => ({
    dataTransfer: { setData: vi.fn(), effectAllowed: "", dropEffect: "" },
  });
  const drag = (from: HTMLElement, onto: HTMLElement) => {
    fireEvent.dragStart(from, transfer());
    fireEvent.dragEnter(onto, transfer());
    fireEvent.dragOver(onto, transfer());
    fireEvent.drop(onto, transfer());
    fireEvent.dragEnd(from, transfer());
  };

  it("moves a request into a folder, and its open tab follows it", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "Open…" }));
    const tree = await screen.findByRole("region", { name: "Demo API" });
    await userEvent.click(within(tree).getByRole("button", { name: /Ping/ }));
    await screen.findByRole("textbox", { name: "URL" });

    drag(
      within(tree).getByRole("button", { name: /Ping/ }),
      within(tree).getByRole("button", { name: /Session/ }),
    );

    await waitFor(() => {
      expect(requestsIn(tree, /Session/)).toEqual(["POSTGet a token", "GETMe", "GETPing"]);
    });
    expect(useTabs.getState().tabs[0]?.file?.path).toBe("session/ping.req.toml");
    expect(useTabs.getState().activeId).toBe(useTabs.getState().tabs[0]?.id);
  });

  it("reorders by dropping after a sibling, and moves a folder into another", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "Open…" }));
    const tree = await screen.findByRole("region", { name: "Demo API" });

    drag(
      within(tree).getByRole("button", { name: /Get a token/ }),
      within(tree).getByRole("button", { name: /Me$/ }),
    );
    await waitFor(() => {
      expect(requestsIn(tree, /Session/)).toEqual(["GETMe", "POSTGet a token"]);
    });

    drag(
      within(tree).getByRole("button", { name: /Users/ }),
      within(tree).getByRole("button", { name: /Session/ }),
    );
    await waitFor(() => {
      const session = within(tree)
        .getByRole("button", { name: /Session/ })
        .closest("li");
      expect(
        within(session as HTMLElement).getByRole("button", { name: /Users/ }),
      ).toBeInTheDocument();
    });
  });

  it("refuses to drop a folder into itself", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "Open…" }));
    const tree = await screen.findByRole("region", { name: "Demo API" });
    const session = within(tree).getByRole("button", { name: /Session/ });

    fireEvent.dragStart(session, transfer());
    const over = fireEvent.dragOver(session, transfer());
    fireEvent.dragEnd(session, transfer());

    // An allowed drop target cancels dragover; this one does not.
    expect(over).toBe(true);
  });
});
