import { clearMocks } from "@tauri-apps/api/mocks";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "../../App";
import { installMockBackend } from "../../test/mockBackend";
import { useCollections } from "../collections/collectionsStore";
import { useTabs } from "../tabs/tabsStore";
import {
  BESIDE_SHARE_INITIAL,
  REQUEST_SHARE,
  SIDEBAR_WIDTH,
  sanitize,
  useLayout,
} from "./layoutStore";

beforeEach(() => {
  localStorage.clear();
  installMockBackend();
  useTabs.setState({ tabs: [], activeId: null });
  useCollections.setState({ views: {} });
  useLayout.setState(sanitize({}));
});

afterEach(async () => {
  cleanup();
  await new Promise((resolve) => setTimeout(resolve, 0));
  clearMocks();
});

describe("layout", () => {
  it("clamps stored sizes and replaces unreadable ones", () => {
    expect(sanitize({ sidebarWidth: 10, requestShare: 2, besideShare: 0 })).toEqual({
      sidebarWidth: SIDEBAR_WIDTH.min,
      split: "below",
      requestShare: REQUEST_SHARE.max,
      besideShare: REQUEST_SHARE.min,
    });
    expect(
      sanitize({ sidebarWidth: Number.NaN, split: "diagonal" as "below", besideShare: Number.NaN }),
    ).toEqual({
      sidebarWidth: SIDEBAR_WIDTH.initial,
      split: "below",
      requestShare: REQUEST_SHARE.initial,
      besideShare: BESIDE_SHARE_INITIAL,
    });
  });

  it("resizes the request, the response and the sidebar, and remembers the sizes", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New request" }));

    const split = screen.getByRole("separator", { name: "Resize the request and the response" });
    fireEvent.keyDown(split, { key: "ArrowDown" });
    expect(split).toHaveAttribute("aria-valuenow", "50");
    expect(screen.getByRole("region", { name: "Request" })).toHaveStyle({ height: "50%" });
    fireEvent.doubleClick(split);
    expect(split).toHaveAttribute("aria-valuenow", "45");

    const sidebar = screen.getByRole("separator", { name: "Resize the sidebar" });
    fireEvent.keyDown(sidebar, { key: "ArrowRight" });
    expect(sidebar).toHaveAttribute("aria-valuenow", String(SIDEBAR_WIDTH.initial + 16));
    expect(JSON.parse(localStorage.getItem("whimbrel.layout") ?? "{}")).toMatchObject({
      sidebarWidth: SIDEBAR_WIDTH.initial + 16,
      requestShare: REQUEST_SHARE.initial,
    });
  });

  it("a request's parts are tabs, chosen with the keyboard and kept per open tab", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New request" }));
    const params = screen.getByRole("tab", { name: "Params", selected: true });
    params.focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(screen.getByRole("tab", { name: "Auth", selected: true })).toHaveFocus();
    await userEvent.keyboard("{End}");
    expect(screen.getByRole("tab", { name: "Settings", selected: true })).toBeInTheDocument();
    expect(screen.getByRole("spinbutton", { name: "Timeout in milliseconds" })).toBeVisible();
    await userEvent.keyboard("{ArrowRight}");
    expect(screen.getByRole("tab", { name: "Params", selected: true })).toBeInTheDocument();
    await userEvent.keyboard("{ArrowLeft}{Home}");
    expect(screen.getByRole("tab", { name: "Params", selected: true })).toBeInTheDocument();

    await userEvent.click(screen.getByRole("tab", { name: "Headers" }));
    act(() => {
      useTabs.getState().newScratch();
    });
    expect(await screen.findByRole("tab", { name: "Params", selected: true })).toBeInTheDocument();
    const first = useTabs.getState().tabs[0];
    if (!first) throw new Error("no first tab");
    act(() => {
      useTabs.getState().activate(first.id);
    });
    expect(await screen.findByRole("tab", { name: "Headers", selected: true })).toBeInTheDocument();
  });

  it("puts the response beside the request, with its own size, and back below", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New request" }));
    await userEvent.click(
      screen.getByRole("button", { name: "Put the response beside the request" }),
    );

    const split = screen.getByRole("separator", { name: "Resize the request and the response" });
    expect(split).toHaveAttribute("aria-orientation", "vertical");
    expect(split).toHaveAttribute("aria-valuenow", "50");
    fireEvent.keyDown(split, { key: "ArrowDown" });
    expect(split).toHaveAttribute("aria-valuenow", "50");
    fireEvent.keyDown(split, { key: "ArrowLeft" });
    expect(split).toHaveAttribute("aria-valuenow", "45");
    expect(screen.getByRole("region", { name: "Request" })).toHaveStyle({ width: "45%" });
    fireEvent.doubleClick(split);
    expect(split).toHaveAttribute("aria-valuenow", "50");
    fireEvent.keyDown(split, { key: "ArrowRight" });

    await userEvent.click(
      screen.getByRole("button", { name: "Put the response below the request" }),
    );
    expect(split).toHaveAttribute("aria-orientation", "horizontal");
    expect(split).toHaveAttribute("aria-valuenow", "45");
    expect(useLayout.getState()).toMatchObject({ split: "below", besideShare: 0.55 });
  });
});
