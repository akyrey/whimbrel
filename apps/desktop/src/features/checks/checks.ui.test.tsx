import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "../../App";
import { runtimeList } from "../../ipc";
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

it("assertions and captures are edited, checked and shown", async () => {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New request" }));
  await userEvent.type(screen.getByRole("textbox", { name: "URL" }), "http://localhost/echo");
  await userEvent.click(screen.getByRole("tab", { name: "Checks" }));
  await userEvent.click(screen.getByRole("button", { name: "Add assertion" }));
  await userEvent.click(screen.getByRole("button", { name: "Add assertion" }));
  await userEvent.selectOptions(screen.getByRole("combobox", { name: "Assertion 2 kind" }), "json");
  const path = screen.getByRole("textbox", { name: "Assertion 2 path" });
  await userEvent.clear(path);
  await userEvent.type(path, "$.method");
  await userEvent.type(screen.getByRole("textbox", { name: "Assertion 2 expected" }), '"POST"');
  await userEvent.click(screen.getByRole("button", { name: "Add assertion" }));
  await userEvent.selectOptions(
    screen.getByRole("combobox", { name: "Assertion 3 kind" }),
    "header",
  );
  await userEvent.type(screen.getByRole("textbox", { name: "Assertion 3 header name" }), "x-mock");
  await userEvent.click(screen.getByRole("button", { name: "Add assertion" }));
  await userEvent.selectOptions(screen.getByRole("combobox", { name: "Assertion 4 kind" }), "time");
  await userEvent.click(screen.getByRole("button", { name: "Add capture" }));
  await userEvent.type(screen.getByRole("textbox", { name: "Capture 1 variable" }), "method");
  const capturePath = screen.getByRole("textbox", { name: "Capture 1 path" });
  await userEvent.clear(capturePath);
  await userEvent.type(capturePath, "$.method");
  await userEvent.click(screen.getByRole("button", { name: "Add capture" }));
  await userEvent.type(screen.getByRole("textbox", { name: "Capture 2 variable" }), "gone");
  await userEvent.selectOptions(
    screen.getByRole("combobox", { name: "Capture 2 source" }),
    "header",
  );
  await userEvent.clear(screen.getByRole("textbox", { name: "Capture 2 path" }));
  await userEvent.type(screen.getByRole("textbox", { name: "Capture 2 path" }), "X-None");
  await userEvent.type(screen.getByRole("textbox", { name: "Capture 2 regex" }), "x");
  await userEvent.click(screen.getByRole("checkbox", { name: "Capture 2 secret" }));

  await userEvent.click(screen.getByRole("button", { name: "Send" }));
  await userEvent.click(await screen.findByRole("tab", { name: "Checks (4/6)" }));

  const checks = screen.getByRole("list", { name: "Checks" });
  expect(within(checks).getByText(/status 200/)).toBeInTheDocument();
  expect(within(checks).getByText(/\$\.method is "GET"/)).toHaveClass("text-red-400");
  expect(within(checks).getByText(/← GET/)).toBeInTheDocument();
  expect(within(checks).getByText(/Nothing found at header X-None/)).toBeInTheDocument();
  expect(await runtimeList()).toEqual([{ name: "method", value: "GET", secret: false }]);

  await userEvent.click(screen.getByRole("button", { name: "Remove capture 2" }));
  await userEvent.click(screen.getByRole("button", { name: "Remove assertion 4" }));
  expect(screen.getByRole("tab", { name: "Checks (4)" })).toBeInTheDocument();
});
