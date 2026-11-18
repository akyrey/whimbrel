import { Toaster, useToasts } from "@roost/ui";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { chooseFolder, toAppError } from "../../ipc";
import { useTabs } from "../tabs/tabsStore";
import {
  closeCollection,
  deleteNode,
  duplicateRequest,
  newFolder,
  openCollection,
  renameRequest,
} from "./actions";
import { useCollections } from "./collectionsStore";
import { NameDialog } from "./NameDialog";

beforeEach(() => {
  useToasts.setState({ toasts: [] });
  useTabs.setState({ tabs: [], activeId: null });
  useCollections.setState({ views: {} });
  localStorage.clear();
});

afterEach(() => {
  clearMocks();
});

describe("when commands fail", () => {
  it("every action says what it could not do", async () => {
    mockIPC(() => Promise.reject({ code: "io", message: "disk full" }));
    render(<Toaster />);

    await act(() => renameRequest("/c", "a.req.toml", "B"));
    await act(() => duplicateRequest("/c", "a.req.toml"));
    await act(() => deleteNode("/c", "a.req.toml"));
    await act(() => newFolder("/c", "", "F"));
    await act(() => closeCollection("/c"));

    const messages = screen.getAllByRole("alert").map((a) => a.textContent);
    expect(messages).toHaveLength(4);
    expect(messages[0]).toContain("disk full");
  });

  it("does nothing when the folder dialog is cancelled", async () => {
    const calls: string[] = [];
    mockIPC((command) => {
      calls.push(command);
      return Promise.resolve(null);
    });

    await openCollection();
    expect(await chooseFolder("x")).toBeNull();

    expect(calls).toEqual(["plugin:dialog|open", "plugin:dialog|open"]);
    expect(useCollections.getState().views).toEqual({});
  });

  it("reports collections that cannot be reopened", async () => {
    localStorage.setItem("whimbrel.collections", JSON.stringify(["/gone"]));
    localStorage.setItem("whimbrel.tabs", "not json");
    mockIPC(() => Promise.reject({ code: "not_found", message: "gone" }));

    expect(await useCollections.getState().restore()).toEqual(["/gone"]);
    await useTabs.getState().restore();
    expect(useTabs.getState().tabs).toEqual([]);
  });

  it("turns anything a command rejects with into an AppError", () => {
    expect(toAppError(new Error("boom"))).toEqual({ code: "internal", message: "boom" });
    expect(toAppError("plain")).toEqual({ code: "internal", message: "plain" });
    expect(toAppError({ code: "x", message: "y" })).toEqual({ code: "x", message: "y" });
  });
});

describe("NameDialog", () => {
  it("needs a name, and can be cancelled", async () => {
    const onSubmit = vi.fn();
    const onCancel = vi.fn();
    render(
      <NameDialog
        title="New folder"
        label="Name"
        initial=""
        confirmLabel="Create"
        onSubmit={onSubmit}
        onCancel={onCancel}
      />,
    );

    expect(screen.getByRole("button", { name: "Create" })).toBeDisabled();
    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "   {Enter}");
    expect(onSubmit).not.toHaveBeenCalled();
    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), " Admin ");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(onSubmit).toHaveBeenCalledWith("Admin");
    expect(onCancel).toHaveBeenCalled();
  });
});
