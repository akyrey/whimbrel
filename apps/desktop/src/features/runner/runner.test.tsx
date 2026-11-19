import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "../../App";
import type { Node } from "../../ipc";
import { installMockBackend } from "../../test/mockBackend";
import { useCollections } from "../collections/collectionsStore";
import { useTabs } from "../tabs/tabsStore";
import { useRunner } from "./runnerStore";
import { EACH, requestsUnder, stepsOf } from "./steps";

beforeEach(() => {
  localStorage.clear();
  installMockBackend();
  useTabs.setState({ tabs: [], activeId: null });
  useCollections.setState({ views: {} });
  useRunner.setState({ target: null });
});

afterEach(async () => {
  cleanup();
  await new Promise((resolve) => setTimeout(resolve, 0));
  clearMocks();
});

async function openRunner(folder: RegExp | "collection") {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open…" }));
  const tree = await screen.findByRole("region", { name: "Demo API" });
  const target =
    folder === "collection"
      ? within(tree).getByRole("heading", { name: "Demo API" })
      : within(tree).getByRole("button", { name: folder });
  fireEvent.contextMenu(target);
  await userEvent.click(screen.getByRole("menuitem", { name: "Run…" }));
}

describe("the runner", () => {
  it("runs a folder: the captured token authorises the next request", async () => {
    await openRunner(/Session/);
    const dialog = screen.getByRole("dialog", { name: "Run Session" });
    expect(within(dialog).getByRole("list", { name: "Steps" })).toHaveTextContent("Get a token");

    await userEvent.click(within(dialog).getByRole("button", { name: "Run" }));

    expect(await within(dialog).findByText("2 passed, 0 failed")).toBeInTheDocument();
    const rows = within(within(dialog).getByRole("table", { name: "Run results" })).getAllByRole(
      "row",
    );
    expect(rows[2]).toHaveTextContent("200");
    await userEvent.click(rows[1] as HTMLElement);
    expect(within(dialog).getByRole("list", { name: "Checks" })).toHaveTextContent(
      "token ← ••••••",
    );
  });

  it("runs a request once per example and stops on the first failure", async () => {
    await openRunner(/Session/);
    const dialog = screen.getByRole("dialog", { name: "Run Session" });
    await userEvent.selectOptions(
      within(dialog).getByRole("combobox", { name: "How to run Me" }),
      EACH,
    );
    await userEvent.click(within(dialog).getByRole("button", { name: "Run" }));
    expect(await within(dialog).findByText("1 passed, 1 failed")).toBeInTheDocument();
    expect(within(dialog).getByText(/› Forged/)).toBeInTheDocument();

    await userEvent.selectOptions(
      within(dialog).getByRole("combobox", { name: "How to run Get a token" }),
      "",
    );
    await userEvent.selectOptions(
      within(dialog).getByRole("combobox", { name: "How to run Me" }),
      "forged",
    );
    await userEvent.click(within(dialog).getByRole("button", { name: "Run" }));
    expect(await within(dialog).findByText("1 passed, 1 failed")).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: "Close" }));
    expect(screen.queryByRole("dialog", { name: "Run Session" })).toBeNull();
  });

  it("runs the whole collection and stops when told", async () => {
    await openRunner("collection");
    const dialog = screen.getByRole("dialog", { name: "Run Demo API" });
    await userEvent.click(
      within(dialog).getByRole("checkbox", { name: "Stop on the first failure" }),
    );
    await userEvent.click(within(dialog).getByRole("button", { name: "Run" }));
    expect(await within(dialog).findByLabelText("Run summary")).toHaveTextContent("passed");
  });
});

describe("steps", () => {
  const nodes: Node[] = [
    { kind: "request", path: "a.req.toml", name: "A", order: 10, method: "GET", examples: [] },
    {
      kind: "folder",
      path: "f",
      name: "F",
      order: 20,
      children: [
        {
          kind: "request",
          path: "f/b.req.toml",
          name: "B",
          order: 10,
          method: "POST",
          examples: [{ id: "e", name: "E" }],
        },
        { kind: "broken", path: "f/x.req.toml", error: "bad" },
      ],
    },
  ];

  it("finds the requests of a folder in tree order", () => {
    expect(requestsUnder(nodes, "").map((r) => r.name)).toEqual(["A", "B"]);
    expect(requestsUnder(nodes, "f").map((r) => r.name)).toEqual(["B"]);
    expect(requestsUnder(nodes, "nope")).toEqual([]);
  });

  it("turns choices into steps", () => {
    const requests = requestsUnder(nodes, "");
    expect(stepsOf(requests, { "f/b.req.toml": "e" })).toEqual([
      { path: "a.req.toml", mode: { kind: "base" } },
      { path: "f/b.req.toml", mode: { kind: "example", id: "e" } },
    ]);
    expect(stepsOf(requests, { "a.req.toml": EACH })[0]?.mode).toEqual({ kind: "each_example" });
  });
});
