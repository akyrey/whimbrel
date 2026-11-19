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

it("the query table and the URL stay in sync both ways", async () => {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New request" }));
  const url = screen.getByRole("textbox", { name: "URL" });
  await userEvent.type(url, "http://localhost/echo?q=ada");

  expect(screen.getByText("Query parameters (1)")).toBeInTheDocument();
  expect(screen.getByRole("textbox", { name: "Query parameter 1 value" })).toHaveValue("ada");

  await userEvent.click(screen.getByRole("button", { name: "Add parameter" }));
  await userEvent.type(screen.getByRole("textbox", { name: "Query parameter 2 name" }), "page");
  await userEvent.type(screen.getByRole("textbox", { name: "Query parameter 2 value" }), "2");
  expect(url).toHaveValue("http://localhost/echo?q=ada&page=2");

  await userEvent.click(screen.getByRole("button", { name: "Remove query parameter 1" }));
  expect(url).toHaveValue("http://localhost/echo?page=2");
});
