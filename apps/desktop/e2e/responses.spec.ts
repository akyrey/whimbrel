import { expect, test } from "@playwright/test";

async function newRequest(page: import("@playwright/test").Page, url: string) {
  await page.goto("/");
  await page.getByRole("button", { name: "New request", exact: true }).click();
  await page.getByRole("textbox", { name: "URL" }).fill(url);
}

/** Phase 4 acceptance: a 10 MB JSON response shows without freezing the window. */
test("a 10 MB JSON response shows without freezing the window", async ({ page }) => {
  await newRequest(page, "http://localhost/huge");
  await page.evaluate(() => {
    const w = window as unknown as { longest: number };
    w.longest = 0;
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) w.longest = Math.max(w.longest, entry.duration);
    }).observe({ type: "longtask", buffered: false });
  });

  const started = Date.now();
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByLabel("Response body")).toContainText('"name":"ada lovelace"', {
    timeout: 15_000,
  });
  const elapsed = Date.now() - started;
  // The window must still answer: typing in the URL field works at once.
  await page.getByRole("textbox", { name: "URL" }).fill("http://localhost/after");
  const longest = await page.evaluate(() => (window as unknown as { longest: number }).longest);

  test.info().annotations.push({
    type: "measured",
    description: `10 MB body shown in ${String(elapsed)} ms; longest main-thread task ${String(Math.round(longest))} ms`,
  });
  expect(elapsed).toBeLessThan(5_000);
  expect(longest).toBeLessThan(1_000);
  await expect(page.getByText("10.0 MB")).toBeVisible();
});

test("history and cURL import", async ({ page }) => {
  await newRequest(page, "http://localhost/users");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByText("200 OK")).toBeVisible();

  await page.getByRole("tab", { name: "History" }).click();
  await expect(
    page.getByRole("navigation", { name: "History" }).getByRole("button", { name: /users/ }),
  ).toBeVisible();

  await page.getByRole("button", { name: "Import cURL…" }).click();
  await page
    .getByRole("textbox", { name: "cURL command" })
    .fill("curl 'http://localhost/echo' -H 'X-From: curl'");
  await page.getByRole("button", { name: "Import", exact: true }).click();
  await expect(page.getByRole("textbox", { name: "URL" })).toHaveValue("http://localhost/echo");
  await expect(page.getByRole("textbox", { name: "Header 1 value" })).toHaveValue("curl");
});
