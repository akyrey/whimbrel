import { expect, test } from "@playwright/test";

test("a collection's OAuth 2: get a token, then requests send it", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Open…" }).click();
  const tree = page.getByRole("region", { name: "Demo API" });
  await tree.getByRole("heading", { name: "Demo API" }).click({ button: "right" });
  await page.getByRole("menuitem", { name: "Auth…" }).click();
  const dialog = page.getByRole("dialog", { name: "Auth of Demo API" });
  await dialog.getByRole("combobox", { name: "Auth type" }).selectOption("oauth2");
  await dialog.getByRole("textbox", { name: "Token URL" }).fill("https://idp.example/token");
  await dialog.getByRole("textbox", { name: "Client ID" }).fill("whimbrel");
  await dialog.getByRole("button", { name: "Save" }).click();

  await tree.getByRole("button", { name: /Ping/ }).click();
  await page.getByText("Auth", { exact: true }).click();
  const panel = page.getByRole("group", { name: "OAuth 2 token" });
  await expect(panel).toContainText("No token yet.");
  await panel.getByRole("button", { name: "Get token" }).click();
  await expect(panel).toContainText("Token ready");

  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByLabel("Response body")).toContainText("Bearer access-1");
});
