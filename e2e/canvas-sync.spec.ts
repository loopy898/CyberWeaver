import { test, expect } from "@playwright/test";

const BROWSER_STORAGE_KEY = "cyberweaver.browser.nodes";

test("syncs tracked clues through the browser persistence fallback", async ({ page }) => {
  await page.goto("/");
  await page.evaluate((key) => {
    window.localStorage.removeItem(key);
  }, BROWSER_STORAGE_KEY);
  await page.reload();

  await expect(page.getByTestId("runtime-mode")).toHaveText("browser");
  await expect(page.getByTestId("sync-status")).toHaveText("Synced", { timeout: 10000 });
  await expect(page.getByTestId("tracked-count-value")).toHaveText("0");

  await page.getByTestId("quick-note").click();

  await expect(page.getByTestId("sync-status")).toHaveText("Synced", { timeout: 10000 });
  await expect(page.getByTestId("tracked-count-value")).toHaveText("1");

  await page.reload();

  await expect(page.getByTestId("tracked-count-value")).toHaveText("1");
});
