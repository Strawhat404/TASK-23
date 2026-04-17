import { test, expect } from '@playwright/test';

const API = process.env.API_URL || 'http://localhost:8000';

test.describe('Authentication flow', () => {
  test('login page loads and shows login form', async ({ page }) => {
    await page.goto('/en/login');
    await expect(page).toHaveURL(/\/en\/login/);
    await expect(page.locator('text=Login').first()).toBeVisible();
  });

  test('login with valid admin credentials succeeds', async ({ page }) => {
    await page.goto('/en/login');
    await page.fill('input[name="username"], input[type="text"]', 'admin');
    await page.fill('input[name="password"], input[type="password"]', 'AdminPass123!');
    await page.click('button[type="submit"], button:has-text("Login")');
    // After login, should redirect away from login page
    await page.waitForURL(/\/en(?!\/login)/, { timeout: 10000 });
    await expect(page).not.toHaveURL(/\/login/);
  });

  test('login with wrong password stays on login page', async ({ page }) => {
    await page.goto('/en/login');
    await page.fill('input[name="username"], input[type="text"]', 'admin');
    await page.fill('input[name="password"], input[type="password"]', 'WrongPassword!');
    await page.click('button[type="submit"], button:has-text("Login")');
    await page.waitForTimeout(2000);
    // Should remain on login page or show error
    await expect(page).toHaveURL(/\/login/);
  });

  test('health API is reachable from browser context', async ({ request }) => {
    const resp = await request.get(`${API}/health/live`);
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(body.success).toBe(true);
    expect(body.data).toBe('alive');
  });

  test('login API returns session cookie', async ({ request }) => {
    const resp = await request.post(`${API}/api/auth/login`, {
      data: { username: 'admin', password: 'AdminPass123!' },
    });
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(body.success).toBe(true);
    expect(body.data.session_cookie).toBeTruthy();
    expect(body.data.user.username).toBe('admin');
    expect(body.data.user.roles).toContain('Admin');
  });
});
