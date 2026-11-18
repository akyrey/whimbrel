import { clearMocks } from "@tauri-apps/api/mocks";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "./App";
import { EMPTY_REQUEST, useRequest } from "./features/request/requestStore";
import { installMockBackend } from "./test/mockBackend";

beforeEach(() => {
  installMockBackend();
  useRequest.setState({ request: EMPTY_REQUEST, status: { kind: "idle" } });
});

afterEach(() => {
  clearMocks();
});

async function typeUrl(url: string) {
  const input = screen.getByRole("textbox", { name: "URL" });
  await userEvent.clear(input);
  await userEvent.type(input, url);
}

describe("App", () => {
  it("sends a request with a header and shows the pretty-printed response", async () => {
    render(<App />);
    await userEvent.clear(screen.getByRole("combobox", { name: "Method" }));
    await userEvent.type(screen.getByRole("combobox", { name: "Method" }), "post");
    await typeUrl("http://localhost/echo");
    await userEvent.click(screen.getByRole("button", { name: "Add header" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Header 1 name" }), "X-Api");
    await userEvent.type(screen.getByRole("textbox", { name: "Header 1 value" }), "secret");

    await userEvent.click(screen.getByRole("button", { name: "Send" }));

    const panel = await screen.findByRole("region", { name: "Response" });
    expect(within(panel).getByText("200 OK")).toBeInTheDocument();
    expect(within(panel).getByText("15 ms")).toBeInTheDocument();
    const body = await within(panel).findByLabelText("Response body");
    expect(body).toHaveTextContent('"method": "POST"');
    expect(body).toHaveTextContent('"X-Api"');
    await userEvent.click(within(panel).getByRole("tab", { name: "Headers (2)" }));
    expect(within(panel).getByText("x-mock")).toBeInTheDocument();
  });

  it("does not send a header that is switched off, and removes one", async () => {
    render(<App />);
    await typeUrl("http://localhost/echo");
    await userEvent.click(screen.getByRole("button", { name: "Add header" }));
    await userEvent.click(screen.getByRole("button", { name: "Add header" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Header 1 name" }), "X-Off");
    await userEvent.click(screen.getByRole("checkbox", { name: "Send header 1" }));
    await userEvent.click(screen.getByRole("button", { name: "Remove header 2" }));

    await userEvent.click(screen.getByRole("button", { name: "Send" }));

    const body = await screen.findByLabelText("Response body");
    expect(body).not.toHaveTextContent("X-Off");
    expect(screen.queryByRole("textbox", { name: "Header 2 name" })).toBeNull();
  });

  it("shows why a request failed", async () => {
    render(<App />);
    await typeUrl("http://unreachable.test/");
    await userEvent.click(screen.getByRole("button", { name: "Send" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Could not connect");
  });

  it("cancels a slow request from the button", async () => {
    render(<App />);
    await typeUrl("http://localhost/slow");
    await userEvent.click(screen.getByRole("button", { name: "Send" }));

    await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Cancelled");
    expect(screen.getByRole("button", { name: "Send" })).toBeInTheDocument();
  });

  it("sends with the keyboard and opens the palette and the shortcuts", async () => {
    render(<App />);
    await typeUrl("http://localhost/missing");

    fireEvent.keyDown(window, { key: "Enter", ctrlKey: true });
    expect(await screen.findByText("404 Not Found")).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    const palette = screen.getByRole("dialog", { name: "Command palette" });
    await userEvent.type(within(palette).getByRole("combobox"), "shortcuts{Enter}");
    expect(screen.getByRole("dialog", { name: "Keyboard shortcuts" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Close" }));

    fireEvent.keyDown(window, { key: "l", ctrlKey: true });
    expect(screen.getByRole("textbox", { name: "URL" })).toHaveFocus();
  });

  it("edits a JSON body in the editor", async () => {
    render(<App />);
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Body type" }), "json");

    expect(await screen.findByLabelText("Request body")).toBeInTheDocument();
    await act(async () => {
      useRequest.getState().setBodyText('{"a":1}');
      await Promise.resolve();
    });
    await waitFor(() => {
      expect(useRequest.getState().request.body).toEqual({
        type: "text",
        kind: "json",
        text: '{"a":1}',
      });
    });
  });
});
