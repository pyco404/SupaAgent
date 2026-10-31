import {test,expect} from '../../web/node_modules/@playwright/test/index.mjs';

test('workspace skip link preserves Home and section views through refresh and history',async({page})=>{
 await page.goto('/?preview=public');await expect(page.locator('html')).toHaveAttribute('data-ready','true');
 await page.locator('.enter').click();await expect(page.locator('body')).toHaveAttribute('data-view','projects');
 await page.locator('.skip').focus();await page.keyboard.press('Enter');
 await expect(page).toHaveURL(/\/\?preview=public#workspace$/);
 await expect(page.locator('#workspace')).toBeFocused();await expect(page.locator('body')).toHaveAttribute('data-view','projects');
 await page.reload();await expect(page.locator('body')).toHaveAttribute('data-view','projects');
 await page.goBack();await expect(page.locator('body')).toHaveAttribute('data-view','projects');
 await page.goBack();await expect(page.locator('body')).toHaveAttribute('data-view','intro');
 await page.goForward();await page.goForward();await expect(page.locator('body')).toHaveAttribute('data-view','projects');
 await page.locator('.command-toggle').click();await page.locator('.command-input').fill('supa memory');await page.keyboard.press('Enter');
 await expect(page).toHaveURL(/\/memory\?preview=public$/);
 await page.locator('.skip').focus();await page.keyboard.press('Enter');await expect(page.locator('#workspace')).toBeFocused();
 await expect(page.locator('.tab-file')).toHaveText('Memory');
 await expect(page).toHaveURL(/\/memory\?preview=public#workspace$/);
});
