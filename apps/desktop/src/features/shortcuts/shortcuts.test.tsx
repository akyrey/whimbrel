import { useToasts } from "@roost/ui";
import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "../../App";
import { installMockBackend } from "../../test/mockBackend";
import { useCollections } from "../collections/collectionsStore";
import { useLayout } from "../layout/layoutStore";
import { setBodyKind, setBodyText } from "../request/editing";
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

/** Renders the app and lets its start-up (restoring tabs) finish before any key is pressed. */
async function start() {
  render(<App />);
  await new Promise((resolve) => setTimeout(resolve, 0));
}

const press = (init: KeyboardEventInit) => {
  fireEvent.keyDown(window, init);
};
const active = () => {
  const { tabs, activeId } = useTabs.getState();
  return tabs.find((t) => t.id === activeId);
};

it("moves to the next and previous tab, wrapping around", async () => {
  await start();
  press({ key: "t", ctrlKey: true });
  press({ key: "t", ctrlKey: true });
  const [first, second] = useTabs.getState().tabs.map((t) => t.id);
  expect(active()?.id).toBe(second);

  press({ key: "Tab", ctrlKey: true });
  expect(active()?.id).toBe(first);
  press({ key: "Tab", ctrlKey: true, shiftKey: true });
  expect(active()?.id).toBe(second);
  press({ key: "ArrowRight", ctrlKey: true, altKey: true });
  expect(active()?.id).toBe(first);
});

it("duplicates the tab's request into a new tab", async () => {
  await start();
  press({ key: "t", ctrlKey: true });
  await userEvent.type(screen.getByRole("textbox", { name: "URL" }), "http://localhost/users");
  const original = active()?.id;

  press({ key: "D", code: "KeyD", ctrlKey: true, shiftKey: true });
  expect(useTabs.getState().tabs).toHaveLength(2);
  expect(active()?.id).not.toBe(original);
  expect(active()?.draft.request.url).toBe("http://localhost/users");
  expect(active()?.draft.name).toMatch(/copy$/);
});

it("focuses the tree filter, switching the sidebar to the collections", async () => {
  await start();
  await userEvent.click(screen.getByRole("tab", { name: "History" }));
  press({ key: "p", ctrlKey: true });
  await waitFor(() => {
    expect(screen.getByRole("searchbox", { name: "Filter requests" })).toHaveFocus();
  });
});

it("beautifies a JSON body, and says when the body is not JSON", async () => {
  await start();
  press({ key: "t", ctrlKey: true });
  const id = active()?.id ?? "";
  press({ key: "B", code: "KeyB", ctrlKey: true, shiftKey: true });
  expect(useToasts.getState().toasts.at(-1)?.message).toMatch(/JSON body/);

  useTabs.getState().edit(id, setBodyKind("json"));
  useTabs.getState().edit(id, setBodyText('{"a":1}'));
  press({ key: "B", code: "KeyB", ctrlKey: true, shiftKey: true });
  const body = active()?.draft.request.body;
  expect(body?.type === "text" && body.text).toBe('{\n  "a": 1\n}');
});

it("puts the response beside or below the request", async () => {
  await start();
  const before = useLayout.getState().split;
  press({ key: "j", ctrlKey: true });
  expect(useLayout.getState().split).not.toBe(before);
  press({ key: "j", ctrlKey: true });
  expect(useLayout.getState().split).toBe(before);
});

it("offers the new shortcuts as palette commands, with their keys", async () => {
  await start();
  press({ key: "k", ctrlKey: true });
  const palette = screen.getByRole("dialog", { name: "Command palette" });
  for (const label of [
    "Next tab",
    "Previous tab",
    "Duplicate the request",
    "Filter the requests",
    "Beautify the JSON body",
    "Put the response beside or below",
  ]) {
    expect(within(palette).getByText(label)).toBeInTheDocument();
  }
});
