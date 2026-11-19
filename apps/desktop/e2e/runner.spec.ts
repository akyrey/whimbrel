import { expect, test } from "@playwright/test";

/** Phase 7 acceptance: login → capture token → authorised call works as a run. */
test("a folder runs: login, capture the token, call with it", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Open…" }).click();
  const tree = page.getByRole("region", { name: "Demo API" });
  await tree.getByRole("button", { name: /Session/ }).click({ button: "right" });
  await page.getByRole("menuitem", { name: "Run…" }).click();

  const dialog = page.getByRole("dialog", { name: "Run Session" });
  await dialog.getByRole("button", { name: "Run" }).click();

  await expect(dialog.getByLabel("Run summary")).toHaveText("2 passed, 0 failed");
  const rows = dialog.getByRole("table", { name: "Run results" }).getByRole("row");
  await expect(rows.nth(2)).toContainText("Me");
  await expect(rows.nth(2)).toContainText("200");
  await expect(rows.nth(2)).toContainText("passed");
});
