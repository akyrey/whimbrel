import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "../../App";
import { installMockBackend } from "../../test/mockBackend";
import { useCollections } from "../collections/collectionsStore";
import { useTabs } from "../tabs/tabsStore";

beforeEach(() => {
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

it("request variables are added, renamed, valued, sent and removed", async () => {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New request" }));
  await userEvent.click(screen.getByText("Variables"));
  await userEvent.click(screen.getByRole("button", { name: "Add variable" }));
  const name = screen.getByRole("textbox", { name: "Variable 1 name" });
  await userEvent.clear(name);
  await userEvent.type(name, "host{Enter}");
  await userEvent.type(screen.getByRole("textbox", { name: "Variable 1 value" }), "localhost");
  await userEvent.type(screen.getByRole("textbox", { name: "URL" }), "http://{{{{host}}/echo");

  await userEvent.click(screen.getByRole("button", { name: "Send" }));

  expect(await screen.findByText("200 OK")).toBeInTheDocument();
  expect(screen.getByText("Variables (1)")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Remove variable host" }));
  expect(screen.queryByRole("textbox", { name: "Variable 1 name" })).toBeNull();
});
