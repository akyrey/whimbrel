import { useToasts } from "@roost/ui";
import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "../../App";
import { installMockBackend } from "../../test/mockBackend";
import { useCollections } from "../collections/collectionsStore";
import { useTabs } from "../tabs/tabsStore";
import { useSpecs } from "./specsStore";

beforeEach(() => {
  useToasts.setState({ toasts: [] });
  localStorage.clear();
  installMockBackend();
  useTabs.setState({ tabs: [], activeId: null });
  useCollections.setState({ views: {} });
  useSpecs.setState({ byRoot: {}, apis: {}, view: null });
});

afterEach(async () => {
  cleanup();
  await new Promise((resolve) => setTimeout(resolve, 0));
  clearMocks();
});

function firstOf(elements: HTMLElement[]): HTMLElement {
  const [first] = elements;
  if (!first) throw new Error("nothing found");
  return first;
}

async function openDialog() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "Open…" }));
  await screen.findByRole("region", { name: "Demo API" });
  await userEvent.click(screen.getByRole("button", { name: "Add or import" }));
  await userEvent.click(screen.getByRole("menuitem", { name: "Import OpenAPI…" }));
  return screen.findByRole("dialog", { name: "Import OpenAPI" });
}

async function importPets() {
  const dialog = await openDialog();
  await userEvent.click(within(dialog).getByRole("button", { name: "Choose file…" }));
  await within(dialog).findByText("/demo");
  await userEvent.click(within(dialog).getByRole("button", { name: "Read" }));
  await within(dialog).findByLabelText("Import summary");
  await userEvent.selectOptions(
    within(dialog).getByRole("combobox", { name: "Import into" }),
    "/demo",
  );
  await userEvent.click(within(dialog).getByRole("button", { name: "Import" }));
  return screen.findByRole("region", { name: "Spec Pets" });
}

describe("OpenAPI import", () => {
  it("imports a file into a folder of the open collection and opens the spec", async () => {
    const spec = await importPets();

    expect(within(spec).getByText("Warnings")).toBeInTheDocument();
    expect(within(spec).getByText("deprecated")).toBeInTheDocument();
    expect(await screen.findByText("Imported 1 request")).toBeInTheDocument();
    const tree = screen.getByRole("region", { name: "Demo API" });
    expect(within(tree).getByText("List pets")).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "API specs" })).toHaveTextContent("Pets");
  });

  it("shows warnings, and a new collection that cannot be made is reported", async () => {
    const dialog = await openDialog();
    await userEvent.click(within(dialog).getByRole("radio", { name: "URL" }));
    await userEvent.type(
      within(dialog).getByRole("textbox", { name: "Spec URL" }),
      "http://x/warn.yaml",
    );
    await userEvent.click(within(dialog).getByRole("button", { name: "Read" }));
    await userEvent.click(await within(dialog).findByText("1 warning"));
    expect(within(dialog).getByText("Only implicit flows")).toBeInTheDocument();

    await userEvent.click(within(dialog).getByRole("button", { name: "Import" }));

    expect(
      await screen.findByText(/Could not import: That folder already holds/),
    ).toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "Import OpenAPI" })).toBeInTheDocument();
  });

  it("editing the source clears the summary", async () => {
    const dialog = await openDialog();
    await userEvent.click(within(dialog).getByRole("radio", { name: "URL" }));
    const url = within(dialog).getByRole("textbox", { name: "Spec URL" });
    await userEvent.type(url, "http://x/a.yaml");
    await userEvent.click(within(dialog).getByRole("button", { name: "Read" }));
    await within(dialog).findByLabelText("Import summary");

    await userEvent.type(url, "x");

    expect(within(dialog).queryByLabelText("Import summary")).toBeNull();
    await userEvent.click(within(dialog).getByRole("radio", { name: "File" }));
    expect(within(dialog).getByText("No file chosen")).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog", { name: "Import OpenAPI" })).toBeNull();
  });
});

describe("spec browser", () => {
  it("browses operations, schemas and webhooks, and goes to the request and back", async () => {
    const spec = await importPets();
    await userEvent.type(within(spec).getByRole("textbox", { name: "Filter operations" }), "pets");
    await userEvent.click(within(spec).getByRole("button", { name: /List pets/ }));
    const operation = within(spec).getByRole("article", { name: "Operation listPets" });
    expect(within(operation).getByRole("cell", { name: "limit" })).toBeInTheDocument();
    const links = within(operation).getByRole("list", { name: "Links" });
    expect(links).toHaveTextContent("limit = $response.body#/0/id");
    expect(links).toHaveTextContent("https://other.test/openapi.json#/paths/~1x/get");
    await userEvent.click(within(operation).getByText("onAdopted"));
    expect(
      within(operation).getByRole("article", { name: "Operation post-request-body-url" }),
    ).toHaveTextContent("{$request.body#/url}");
    await userEvent.click(within(links).getByRole("button", { name: "listPets" }));
    await userEvent.click(firstOf(within(operation).getAllByRole("button", { name: "Pet" })));

    const schema = within(spec).getByLabelText("Schema of Pet");
    expect(schema).toHaveTextContent('one of "cat", "dog"');
    expect(schema).toHaveTextContent("minLength 1");
    await userEvent.click(within(schema).getByRole("button", { name: "Expand tags" }));
    expect(schema).toHaveTextContent("[items]");
    await userEvent.click(within(schema).getByRole("button", { name: "Collapse Pet" }));
    expect(schema).not.toHaveTextContent("owner");

    await userEvent.click(within(spec).getByText("Webhooks"));
    await userEvent.click(within(spec).getByRole("button", { name: /A pet was added/ }));
    const hook = within(spec).getByRole("article", { name: "Operation petAdded" });
    expect(within(hook).getByText("sent by the API")).toBeInTheDocument();
    expect(within(hook).queryByRole("button", { name: "Open request" })).toBeNull();
    await userEvent.click(within(hook).getByRole("tab", { name: "text/event-stream" }));
    expect(within(hook).getByLabelText("Schema of each item")).toBeInTheDocument();

    await userEvent.click(within(spec).getByText("Schemas"));
    await userEvent.click(firstOf(within(spec).getAllByRole("button", { name: "Pet" })));
    await userEvent.click(within(spec).getByRole("button", { name: "Overview" }));
    expect(within(spec).getByText("A small pet store.")).toBeInTheDocument();

    useSpecs.getState().focus({ kind: "operation", id: "listPets" });
    await userEvent.click(await within(spec).findByRole("button", { name: "Open request" }));
    await waitFor(() => {
      expect(screen.getByRole("textbox", { name: "URL" })).toHaveValue("{{baseUrl}}/pets");
    });
    await userEvent.click(screen.getByRole("button", { name: "Show in spec" }));
    expect(await screen.findByRole("article", { name: "Operation listPets" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: /List pets/ }));
    expect(screen.queryByRole("region", { name: "Spec Pets" })).toBeNull();

    useSpecs.getState().focus({ kind: "operation", id: "gone" });
    await userEvent.click(
      within(screen.getByRole("navigation", { name: "API specs" })).getByRole("button", {
        name: "Pets",
      }),
    );
    expect(await screen.findByText("A small pet store.")).toBeInTheDocument();
    useSpecs.getState().focus({ kind: "operation", id: "gone" });
    expect(await screen.findByText("This operation is no longer in the spec.")).toBeInTheDocument();
  });

  it("validates the last response of a generated request against the spec", async () => {
    const spec = await importPets();
    useSpecs.getState().focus({ kind: "operation", id: "listPets" });
    await userEvent.click(await within(spec).findByRole("button", { name: "Open request" }));
    await screen.findByRole("textbox", { name: "URL" });
    expect(screen.queryByRole("button", { name: "Validate against the spec" })).toBeNull();

    await userEvent.click(screen.getByRole("button", { name: "Send" }));
    await userEvent.click(await screen.findByRole("button", { name: "Validate against the spec" }));

    const section = screen.getByRole("region", { name: "Spec validation" });
    expect(
      await within(section).findByText(/1 difference from the spec \(200 application\/json\)/),
    ).toBeInTheDocument();
    expect(within(section).getByRole("list", { name: "Schema errors" })).toHaveTextContent(
      'not of type "array"',
    );
  });

  it("re-imports: compares, lets a conflict take the spec's side, applies", async () => {
    const spec = await importPets();
    await userEvent.click(within(spec).getByRole("button", { name: "Re-import…" }));
    const dialog = screen.getByRole("dialog", { name: "Re-import Pets" });
    const location = within(dialog).getByRole("textbox", { name: "New spec location" });
    expect(location).toHaveValue("/demo");

    await userEvent.clear(location);
    await userEvent.type(location, "http://localhost/same.yaml");
    await userEvent.click(within(dialog).getByRole("button", { name: "Compare" }));
    expect(
      await within(dialog).findByText("Nothing changed since the last import."),
    ).toBeInTheDocument();

    await userEvent.clear(location);
    await userEvent.type(location, "/specs/pets-v2.yaml");
    await userEvent.click(within(dialog).getByRole("button", { name: "Compare" }));
    const changes = await within(dialog).findByRole("list", { name: "Re-import changes" });
    expect(changes).toHaveTextContent("createPet");
    expect(changes).toHaveTextContent("Left the spec");
    expect(within(dialog).getByText(/gonePet/)).toBeInTheDocument();
    const conflict = within(dialog).getByRole("group", { name: "listPets name" });
    expect(within(conflict).getByRole("radio", { name: /Keep mine: My pets/ })).toBeChecked();
    await userEvent.click(within(conflict).getByRole("radio", { name: /Take the spec's/ }));

    await userEvent.click(within(dialog).getByRole("button", { name: "Apply" }));

    expect(await screen.findByText("Re-imported: 3 changes")).toBeInTheDocument();
    expect(screen.queryByRole("dialog", { name: "Re-import Pets" })).toBeNull();
  });

  it("forgets the specs of a closed collection", async () => {
    await importPets();
    useSpecs.getState().forget("/demo");
    expect(useSpecs.getState().view).toBeNull();
    expect(useSpecs.getState().byRoot).toEqual({});
  });
});
