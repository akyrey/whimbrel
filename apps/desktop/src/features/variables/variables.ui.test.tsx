import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "../../App";
import { installMockBackend } from "../../test/mockBackend";
import { useCollections } from "../collections/collectionsStore";
import { useEnvironments } from "../environments/environmentsStore";
import { useTabs } from "../tabs/tabsStore";

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

it("{{ completes variables in scope and the used ones explain themselves", async () => {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open…" }));
  const tree = await screen.findByRole("region", { name: "Demo API" });
  await userEvent.selectOptions(
    within(tree).getByRole("combobox", { name: /environment/i }),
    "local",
  );
  await userEvent.click(within(tree).getByRole("button", { name: /Ping/ }));
  const url = await screen.findByRole("textbox", { name: "URL" });

  await userEvent.clear(url);
  await userEvent.type(url, "{{{{ba");
  const list = await screen.findByRole("listbox", { name: "Variables" });
  expect(within(list).getByRole("option", { name: /base/ })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await userEvent.keyboard("{Enter}");
  await userEvent.type(url, "/x?u={{{{nope}}");

  expect(url).toHaveValue("{{base}}/x?u={{nope}}");
  const used = screen.getByRole("list", { name: "Variables used" });
  await waitFor(() => {
    expect(
      within(used).getByLabelText("base = http://localhost (from the environment)"),
    ).toBeInTheDocument();
  });
  expect(within(used).getByLabelText("nope is not defined")).toBeInTheDocument();

  await userEvent.type(url, "&t={{{{$");
  expect(await screen.findByRole("listbox", { name: "Variables" })).toHaveTextContent("$uuid");
  await userEvent.keyboard("{ArrowDown}{ArrowUp}{Escape}");
  expect(screen.queryByRole("listbox", { name: "Variables" })).toBeNull();
});

/** Places `element` at a fixed rectangle, as layout would (jsdom has none). */
function placeAt(element: Element, left: number, width: number): void {
  const rect = {
    left,
    right: left + width,
    top: 10,
    bottom: 30,
    width,
    height: 20,
    x: left,
    y: 10,
  };
  element.getBoundingClientRect = () => ({ ...rect, toJSON: () => rect });
}

it("hovering a variable in the URL or in the used row shows its value and source", async () => {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open…" }));
  const tree = await screen.findByRole("region", { name: "Demo API" });
  await userEvent.selectOptions(
    within(tree).getByRole("combobox", { name: /environment/i }),
    "local",
  );
  await userEvent.click(within(tree).getByRole("button", { name: /Ping/ }));
  const url = await screen.findByRole("textbox", { name: "URL" });
  await userEvent.clear(url);
  await userEvent.type(url, "{{{{base}}/x?u={{{{nope}}");

  const base = document.querySelector('[data-variable="base"]');
  const nope = document.querySelector('[data-variable="nope"]');
  if (!base || !nope) throw new Error("variables are not marked in the URL");
  placeAt(base, 0, 60);
  placeAt(nope, 100, 60);
  await waitFor(() => {
    expect(base).toHaveAttribute("data-known", "true");
  });
  expect(nope).toHaveAttribute("data-known", "false");

  fireEvent.mouseMove(url, { clientX: 20, clientY: 20 });
  const tip = await screen.findByRole("tooltip");
  expect(tip).toHaveTextContent("base");
  expect(tip).toHaveTextContent("http://localhost");
  expect(tip).toHaveTextContent("From the environment");

  fireEvent.mouseMove(url, { clientX: 120, clientY: 20 });
  expect(screen.getByRole("tooltip")).toHaveTextContent("Not defined here");
  fireEvent.mouseMove(url, { clientX: 80, clientY: 20 });
  expect(screen.queryByRole("tooltip")).toBeNull();

  const used = screen.getByRole("list", { name: "Variables used" });
  await userEvent.hover(within(used).getByText("base"));
  expect(screen.getByRole("tooltip")).toHaveTextContent("http://localhost");
  await userEvent.unhover(within(used).getByText("base"));
  expect(screen.queryByRole("tooltip")).toBeNull();
});
