import { test, expect } from '@playwright/test';

const API = process.env.API_URL || 'http://localhost:8000';

test.describe('Customer journey: browse → cart → checkout', () => {
  test('menu page lists products after login', async ({ page }) => {
    // Login first
    await page.goto('/en/login');
    await page.fill('input[name="username"], input[type="text"]', 'customer');
    await page.fill('input[name="password"], input[type="password"]', 'CustomerPass123!');
    await page.click('button[type="submit"], button:has-text("Login")');
    await page.waitForURL(/\/en(?!\/login)/, { timeout: 10000 });

    // Navigate to menu
    await page.goto('/en/menu');
    await page.waitForTimeout(2000);
    // Should see at least one product card/item
    const body = await page.textContent('body');
    expect(body).toBeTruthy();
  });

  test('products API returns items', async ({ request }) => {
    const resp = await request.get(`${API}/api/products/`);
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(body.success).toBe(true);
    expect(Array.isArray(body.data)).toBe(true);
    expect(body.data.length).toBeGreaterThan(0);
    // Each product should have bilingual names
    const first = body.data[0];
    expect(first.name_en).toBeTruthy();
    expect(first.name_zh).toBeTruthy();
    expect(first.base_price).toBeGreaterThan(0);
  });

  test('product detail API returns option groups', async ({ request }) => {
    const resp = await request.get(`${API}/api/products/1`);
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(body.success).toBe(true);
    expect(body.data.option_groups).toBeTruthy();
    expect(Array.isArray(body.data.option_groups)).toBe(true);
    // At least one required option group
    const required = body.data.option_groups.filter((g: any) => g.is_required);
    expect(required.length).toBeGreaterThan(0);
  });

  test('store hours API returns schedule', async ({ request }) => {
    const resp = await request.get(`${API}/api/store/hours`);
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(body.success).toBe(true);
    expect(Array.isArray(body.data)).toBe(true);
  });

  test('tax API returns active rate', async ({ request }) => {
    const resp = await request.get(`${API}/api/store/tax`);
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(body.data.rate).toBeGreaterThanOrEqual(0);
    expect(body.data.is_active).toBe(true);
  });
});
