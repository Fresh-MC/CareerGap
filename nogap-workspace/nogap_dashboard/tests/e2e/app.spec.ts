import { test, expect } from '@playwright/test';

test('homepage has title and links to the CLI documentation', async ({ page }) => {
    await page.goto('http://localhost:3000'); // Adjust the URL as needed for your local setup

    // Check the title of the page
    await expect(page).toHaveTitle(/NoGap Dashboard/);

    // Check if the CLI documentation link is present
    const cliLink = page.locator('a[href="/cli"]'); // Adjust the selector as needed
    await expect(cliLink).toBeVisible();
});

test('can navigate to the features page', async ({ page }) => {
    await page.goto('http://localhost:3000');

    // Click on the features link
    await page.click('a[href="/features"]'); // Adjust the selector as needed

    // Check if the features page is displayed
    await expect(page).toHaveURL(/.*features/);
    await expect(page.locator('h1')).toHaveText('Features'); // Adjust the selector as needed
});