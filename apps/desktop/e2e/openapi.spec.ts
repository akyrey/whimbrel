import { expect, test } from "@playwright/test";

/** Phase 6: import a spec into an open collection, browse it, and go both ways between the
 * spec and the generated requests. */
test("import an OpenAPI spec and browse it", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Open…" }).click();
  await expect(page.getByRole("region", { name: "Demo API" })).toBeVisible();

  await page.getByRole("button", { name: "Add or import" }).click();
  await page.getByRole("menuitem", { name: "Import OpenAPI…" }).click();
  await page.getByRole("radio", { name: "URL" }).check();
  await page.getByRole("textbox", { name: "Spec URL" }).fill("http://localhost/remote.yaml");
  await page.getByRole("button", { name: "Read", exact: true }).click();
  const summary = page.getByLabel("Import summary");
  await expect(summary).toContainText("Pets");
  await expect(summary).toContainText("1 requests in Pets");
  await expect(page.getByText("http://other.test/schemas.yaml")).toBeVisible();
  await page.getByRole("button", { name: "Fetch them" }).click();
  await expect(page.getByText("http://other.test/schemas.yaml")).toBeHidden();
  await page
    .getByRole("combobox", { name: "Import into" })
    .selectOption({ label: "Demo API (a new folder)" });
  await page.getByRole("button", { name: "Import", exact: true }).click();

  const spec = page.getByRole("region", { name: "Spec Pets" });
  await expect(spec).toBeVisible();
  await expect(spec.getByText("A small pet store.")).toBeVisible();
  await spec
    .getByRole("list", { name: "Operations by tag" })
    .getByText("Pets", { exact: true })
    .click();
  await spec.getByRole("button", { name: /List pets/ }).click();
  const operation = spec.getByRole("article", { name: "Operation listPets" });
  await expect(operation.getByRole("cell", { name: "limit" })).toBeVisible();
  await operation.getByRole("button", { name: "Pet", exact: true }).click();
  await expect(spec.getByLabel("Schema of Pet")).toContainText("required");

  await spec.getByRole("button", { name: /List pets/ }).click();
  await operation.getByRole("button", { name: "Open request" }).click();
  await expect(page.getByRole("textbox", { name: "URL" })).toHaveValue("{{baseUrl}}/pets");
  await page.getByRole("button", { name: "Show in spec" }).click();
  await expect(page.getByRole("article", { name: "Operation listPets" })).toBeVisible();
  await page.getByRole("button", { name: "Close spec" }).click();
  await expect(page.getByRole("textbox", { name: "URL" })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "API specs" })).toContainText("Pets");
});

test("a document that is not OpenAPI is refused before importing", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Add or import" }).click();
  await page.getByRole("menuitem", { name: "Import OpenAPI…" }).click();
  await page.getByRole("radio", { name: "URL" }).check();
  await page.getByRole("textbox", { name: "Spec URL" }).fill("http://localhost/broken.yaml");
  await page.getByRole("button", { name: "Read", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("not an OpenAPI document");
  await expect(page.getByRole("button", { name: "Import", exact: true })).toBeHidden();
});
