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
