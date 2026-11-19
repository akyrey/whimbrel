import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, render, screen, within } from "@testing-library/react";
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

async function newRequest() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New request" }));
  await userEvent.type(screen.getByRole("textbox", { name: "URL" }), "http://localhost/echo");
}

const sentBody = async () => {
  await userEvent.click(screen.getByRole("button", { name: "Send" }));
  await screen.findByText("200 OK");
  return (await screen.findByLabelText("Response body")).textContent;
};

it("a multipart body sends text fields and files", async () => {
  await newRequest();
  await userEvent.selectOptions(screen.getByRole("combobox", { name: "Body type" }), "multipart");
  const fields = screen.getByRole("group", { name: "Form fields" });
  await userEvent.click(within(fields).getByRole("button", { name: "Add field" }));
  await userEvent.type(within(fields).getByRole("textbox", { name: "Field 1 name" }), "title");
  await userEvent.type(within(fields).getByRole("textbox", { name: "Field 1 value" }), "Hello");
  await userEvent.click(within(fields).getByRole("button", { name: "Add field" }));
  await userEvent.type(within(fields).getByRole("textbox", { name: "Field 2 name" }), "doc");
  await userEvent.selectOptions(
    within(fields).getByRole("combobox", { name: "Field 2 type" }),
    "file",
  );
  await userEvent.click(within(fields).getByRole("button", { name: "Field 2 file" }));
  expect(await within(fields).findByDisplayValue("/demo")).toBeInTheDocument();
  await userEvent.click(within(fields).getByRole("button", { name: "Add field" }));
  await userEvent.click(within(fields).getByRole("checkbox", { name: "Send field 3" }));
  await userEvent.click(within(fields).getByRole("button", { name: "Remove field 3" }));

  expect(await sentBody()).toContain("title=Hello&doc=@/demo");
});

it("a form and a file body", async () => {
  await newRequest();
  await userEvent.selectOptions(screen.getByRole("combobox", { name: "Body type" }), "form");
  const fields = screen.getByRole("group", { name: "Form fields" });
  expect(within(fields).queryByRole("combobox", { name: "Field 1 type" })).toBeNull();
  await userEvent.click(within(fields).getByRole("button", { name: "Add field" }));
  await userEvent.type(within(fields).getByRole("textbox", { name: "Field 1 name" }), "q");
  await userEvent.type(within(fields).getByRole("textbox", { name: "Field 1 value" }), "ada");
  expect(await sentBody()).toContain("q=ada");

  await userEvent.selectOptions(screen.getByRole("combobox", { name: "Body type" }), "file");
  await userEvent.type(screen.getByRole("textbox", { name: "Body file path" }), "/tmp/a.bin");
  expect(await sentBody()).toContain("file /tmp/a.bin");
});
