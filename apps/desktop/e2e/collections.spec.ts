import { expect, test } from "@playwright/test";

type MockWindow = {
  whimbrelMock: {
    externalEdit: (
      path: string,
      change: ((s: { request: { url: string } }) => unknown) | null,
    ) => Promise<void>;
  };
};

test("open a collection, edit a request, save it", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Open…" }).click();
  const tree = page.getByRole("region", { name: "Demo API" });
  await tree.getByRole("button", { name: /List users/ }).click();

  await page.getByRole("textbox", { name: "URL" }).fill("http://localhost/echo?page=2");
  await expect(page.getByRole("tab", { name: /List users/ })).toContainText("•");
  await page.keyboard.press("ControlOrMeta+s");
  await expect(page.getByRole("tab", { name: /List users/ })).not.toContainText("•");

  await page.reload();
  await expect(page.getByRole("tab", { name: /List users/ })).toBeVisible();
});

test("create, rename and delete a request from the menus", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Open…" }).click();
  const tree = page.getByRole("region", { name: "Demo API" });

  await tree.getByRole("heading", { name: "Demo API" }).click({ button: "right" });
  await page.getByRole("menuitem", { name: "New request" }).click();
  await page.getByRole("textbox", { name: "Name" }).fill("Login");
  await page.keyboard.press("Enter");
  await expect(tree.getByRole("button", { name: /Login/ })).toBeVisible();

  await tree.getByRole("button", { name: /Login/ }).click({ button: "right" });
  await page.getByRole("menuitem", { name: "Rename" }).click();
  await page.getByRole("textbox", { name: "Name" }).fill("Sign in");
  await page.keyboard.press("Enter");
  await expect(tree.getByRole("button", { name: /Sign in/ })).toBeVisible();

  await tree.getByRole("button", { name: /Sign in/ }).click({ button: "right" });
  await page.getByRole("menuitem", { name: "Delete" }).click();
  await page.getByRole("button", { name: "Delete" }).click();
  await expect(tree.getByRole("button", { name: /Sign in/ })).toHaveCount(0);
});

test("a change on disk under unsaved edits asks which to keep", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Open…" }).click();
  await page
    .getByRole("region", { name: "Demo API" })
    .getByRole("button", { name: /Ping/ })
    .click();
  await page.getByRole("textbox", { name: "URL" }).fill("http://mine");

  await page.evaluate(() =>
    (window as unknown as MockWindow).whimbrelMock.externalEdit("ping.req.toml", (s) => ({
      ...s,
      request: { ...s.request, url: "http://from-git" },
    })),
  );

  await expect(
    page.getByText("This request changed on disk while you were editing it."),
  ).toBeVisible();
  await page.getByRole("button", { name: "Keep mine" }).click();
  await expect(page.getByRole("textbox", { name: "URL" })).toHaveValue("http://mine");
  await page.keyboard.press("ControlOrMeta+s");
  await expect(page.getByRole("tab", { name: /Ping/ })).not.toContainText("•");
});
