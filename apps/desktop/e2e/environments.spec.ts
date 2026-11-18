import { expect, test } from "@playwright/test";

async function openPing(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.getByRole("button", { name: "Open…" }).click();
  await page
    .getByRole("region", { name: "Demo API" })
    .getByRole("button", { name: /Ping/ })
    .click();
}

test("an environment's variables resolve, and the preview says what is missing", async ({
  page,
}) => {
  await openPing(page);
  await page.getByRole("textbox", { name: "URL" }).fill("{{base}}/echo");

  await page.getByRole("button", { name: "Preview" }).click();
  await expect(page.getByRole("dialog", { name: "Resolved request" })).toContainText(
    "Not defined: base",
  );
  await page.getByRole("button", { name: "Close", exact: true }).click();

  await page.getByRole("combobox", { name: "Environment" }).selectOption({ label: "Local" });
  await page.getByRole("button", { name: "Preview" }).click();
  await expect(page.getByLabel("Request as sent")).toContainText("GET http://localhost/echo");
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByText("200 OK")).toBeVisible();
});

test("a secret is set once, masked, and sending to production asks first", async ({ page }) => {
  await openPing(page);
  await page.getByRole("textbox", { name: "URL" }).fill("{{base}}/echo");
  await page.getByRole("button", { name: "Add header" }).click();
  await page.getByRole("textbox", { name: "Header 1 name" }).fill("X-Api-Key");
  await page.getByRole("textbox", { name: "Header 1 value" }).fill("{{apiKey}}");
  await page
    .getByRole("combobox", { name: "Environment" })
    .selectOption({ label: "Production 🔒" });

  await page.getByRole("button", { name: "Edit environments" }).click();
  const editor = page.getByRole("dialog", { name: "Environments" });
  await editor.getByRole("button", { name: /Production/ }).click();
  await editor.getByLabel("Value of apiKey").fill("sk-live-e2e");
  await editor.getByRole("button", { name: "Set" }).click();
  await expect(editor.getByText("set", { exact: true })).toBeVisible();
  await editor.getByRole("button", { name: "Close", exact: true }).last().click();

  await page.getByRole("button", { name: "Preview" }).click();
  await expect(page.getByLabel("Request as sent")).toContainText("X-Api-Key: ••••••");
  await expect(page.getByLabel("Request as sent")).not.toContainText("sk-live-e2e");
  await page.getByRole("button", { name: "Close", exact: true }).click();

  await page.getByRole("button", { name: "Send" }).click();
  const confirm = page.getByRole("dialog", { name: /protected environment/ });
  await expect(confirm).toBeVisible();
  await confirm.getByRole("button", { name: "Send" }).click();
  await expect(page.getByText("200 OK")).toBeVisible();
});

test("save the edits as an example and send with it", async ({ page }) => {
  await openPing(page);
  await page.getByRole("textbox", { name: "URL" }).fill("http://localhost/echo?page=2");
  await page.getByRole("button", { name: "Save as example…" }).click();
  await page.getByRole("textbox", { name: "Example name" }).fill("Page 2");
  await page.keyboard.press("Enter");

  await expect(
    page.getByRole("region", { name: "Demo API" }).getByRole("button", { name: /Page 2/ }),
  ).toBeVisible();
  await expect(page.getByRole("textbox", { name: "URL" })).toHaveValue("http://localhost/echo");
  await page.getByRole("button", { name: "Preview" }).click();
  await expect(page.getByLabel("Request as sent")).toContainText("echo?page=2");
});
