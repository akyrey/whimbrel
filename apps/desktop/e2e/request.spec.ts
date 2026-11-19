import { expect, test } from "@playwright/test";

test("send a POST with a JSON body and read the response", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "New request", exact: true }).click();
  await page.getByRole("combobox", { name: "Method" }).fill("POST");
  await page.getByRole("textbox", { name: "URL" }).fill("http://localhost/echo");
  await page
    .getByRole("tablist", { name: "Request parts" })
    .getByRole("tab", { name: /^Body/ })
    .click();
  await page.getByRole("combobox", { name: "Body type" }).selectOption("json");
  const editor = page.getByLabel("Request body");
  await editor.click();
  await page.keyboard.type('{"name": "ada"}');
  await page.getByRole("button", { name: "Beautify" }).click();
  await expect(page.getByLabel("Request body").locator(".cm-line")).toHaveText([
    "{",
    '  "name": "ada"',
    "}",
  ]);
  await page.getByLabel("Request body").click();
  await page.keyboard.press("ControlOrMeta+z");
  await expect(page.getByLabel("Request body").locator(".cm-line")).toHaveText(['{"name": "ada"}']);
  await page.getByRole("button", { name: "Beautify" }).click();

  await page.getByRole("button", { name: "Send" }).click();

  const response = page.getByRole("region", { name: "Response" });
  await expect(response.getByText("200 OK")).toBeVisible();
  await expect(response.getByLabel("Response body")).toContainText('"method": "POST"');
  await expect(response.getByLabel("Response body")).toContainText("ada");
});

test("cancel a slow request, then send another", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "New request", exact: true }).click();
  await page.getByRole("textbox", { name: "URL" }).fill("http://localhost/slow");
  await page.getByRole("button", { name: "Send" }).click();
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(page.getByRole("alert")).toContainText("Cancelled");

  await page.getByRole("textbox", { name: "URL" }).fill("http://localhost/missing");
  await page.keyboard.press("ControlOrMeta+Enter");
  await expect(page.getByText("404 Not Found")).toBeVisible();
});

test("an invalid URL is explained", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "New request", exact: true }).click();
  await page.getByRole("textbox", { name: "URL" }).fill("localhost/users");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByRole("alert")).toContainText("The URL needs http:// or https://");
});

test("the palette runs commands", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "New request", exact: true }).click();
  await page.keyboard.press("ControlOrMeta+k");
  await page.getByRole("combobox", { name: "Search commands" }).fill("shortcuts");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog", { name: "Keyboard shortcuts" })).toBeVisible();
});
