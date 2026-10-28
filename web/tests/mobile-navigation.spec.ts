import { expect, test } from "@playwright/test";

test.use({ viewport: { width: 390, height: 844 } });

test("full-screen menu locks scrolling, traps focus and restores it on Escape", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => window.scrollTo({ top: 260, behavior: "instant" }));
  const scroll = await page.evaluate(() => window.scrollY);
  const trigger = page.getByRole("button", { name: "MENU +" });
  await trigger.evaluate(element => (element as HTMLButtonElement).click());
  const dialog = page.getByRole("dialog", { name: "Mobile navigation" });
  await expect(dialog).toBeVisible();
  expect(await dialog.boundingBox()).toEqual({ x: 0, y: 0, width: 390, height: 844 });
  await expect(page.getByRole("button", { name: "Close navigation" })).toBeFocused();
  await expect(trigger).toHaveAttribute("aria-expanded", "true");
  expect(await page.evaluate(() => getComputedStyle(document.body).position)).toBe("fixed");
  await dialog.getByRole("link", { name: /Factory/ }).focus();
  await page.keyboard.press("Tab");
  await expect(dialog.getByRole("link", { name: "Supa Agent home" })).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(dialog.getByRole("link", { name: /Factory/ })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(dialog).not.toBeVisible();
  await expect(trigger).toBeFocused();
  await expect(trigger).toHaveAttribute("aria-expanded", "false");
  expect(await page.evaluate(() => window.scrollY)).toBe(scroll);
  expect(await page.evaluate(() => getComputedStyle(document.body).position)).not.toBe("fixed");
});

test("selected links and browser Back close the overlay and highlight the active page", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "MENU +" }).click();
  const dialog = page.getByRole("dialog", { name: "Mobile navigation" });
  await dialog.getByRole("link", { name: /Memory/ }).click();
  await expect(page).toHaveURL(/\/memory$/);
  await expect(dialog).not.toBeVisible();
  await page.getByRole("button", { name: "MENU +" }).click();
  await expect(dialog.getByRole("link", { name: /Memory/ })).toHaveAttribute("aria-current", "page");
  await page.goBack();
  await expect(page).toHaveURL(/\/$/);
  await expect(dialog).not.toBeVisible();
});

test("close button and desktop resize release modality without changing desktop navigation", async ({ page }) => {
  await page.goto("/agents");
  const trigger = page.getByRole("button", { name: "MENU +" });
  await trigger.click();
  await page.getByRole("button", { name: "Close navigation" }).click();
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await trigger.click();
  await page.setViewportSize({ width: 1440, height: 1000 });
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await expect(trigger).not.toBeVisible();
  await expect(page.getByRole("navigation", { name: "Main navigation" })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Main navigation" }).getByRole("link", { name: /Agents/ })).toHaveAttribute("aria-current", "page");
  expect(await page.evaluate(() => getComputedStyle(document.body).position)).not.toBe("fixed");
});

for (const viewport of [{ width: 320, height: 568 }, { width: 390, height: 844 }, { width: 640, height: 800 }]) {
  test(`overlay fits ${viewport.width}×${viewport.height} and every item remains reachable`, async ({ page }) => {
    await page.setViewportSize(viewport); await page.goto("/");
    await page.getByRole("button", { name: "MENU +" }).click();
    const dialog = page.getByRole("dialog", { name: "Mobile navigation" });
    await expect(dialog).toBeVisible();
    expect(await dialog.boundingBox()).toEqual({ x: 0, y: 0, ...viewport });
    expect(await dialog.evaluate(element => element.scrollWidth <= element.clientWidth)).toBe(true);
    await dialog.getByRole("link", { name: /Factory/ }).scrollIntoViewIfNeeded();
    await expect(dialog.getByRole("link", { name: /Factory/ })).toBeInViewport();
    await expect(page.getByRole("button", { name: "Close navigation" })).toBeInViewport();
    await page.keyboard.press("Escape"); await expect(dialog).not.toBeVisible();
  });
}

test("motion is subtle and reduced-motion preference removes animation", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" }); await page.goto("/");
  await page.getByRole("button", { name: "MENU +" }).click();
  const dialog = page.getByRole("dialog", { name: "Mobile navigation" });
  expect(await dialog.evaluate(e => getComputedStyle(e).animationName)).toBe("mobile-menu-enter");
  await page.getByRole("button", { name: "Close navigation" }).click();
  await expect(dialog).toHaveAttribute("data-closing", "true");
  await expect(dialog).not.toBeVisible();
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.getByRole("button", { name: "MENU +" }).click();
  expect(await dialog.evaluate(e => getComputedStyle(e).animationName)).toBe("none");
});
