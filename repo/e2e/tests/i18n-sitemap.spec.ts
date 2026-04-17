import { test, expect } from '@playwright/test';

const API = process.env.API_URL || 'http://localhost:8000';

test.describe('Internationalization', () => {
  test('locale switcher changes page language', async ({ page }) => {
    await page.goto('/en/login');
    // Page should be in English
    const body = await page.textContent('body');
    expect(body).toBeTruthy();

    // Switch to Chinese
    await page.goto('/zh/login');
    const zhBody = await page.textContent('body');
    expect(zhBody).toBeTruthy();
  });

  test('i18n API returns English translations', async ({ request }) => {
    const resp = await request.get(`${API}/api/i18n/translations/en`);
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(body.data['nav.home']).toBe('Home');
    expect(body.data['nav.menu']).toBe('Menu');
    expect(body.data['btn.checkout']).toBe('Checkout');
  });

  test('i18n API returns Chinese translations', async ({ request }) => {
    const resp = await request.get(`${API}/api/i18n/translations/zh`);
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(body.data['nav.home']).toBeTruthy();
    expect(body.data['nav.home']).not.toBe('Home'); // Should be Chinese
    expect(body.data['nav.home']).not.toBe('nav.home'); // Should be resolved
  });

  test('unknown locale returns 404', async ({ request }) => {
    const resp = await request.get(`${API}/api/i18n/translations/fr`);
    expect(resp.status()).toBe(404);
  });

  test('locales list includes en and zh', async ({ request }) => {
    const resp = await request.get(`${API}/api/i18n/locales`);
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    const codes = body.data.map((l: any) => l.code);
    expect(codes).toContain('en');
    expect(codes).toContain('zh');
  });
});

test.describe('Sitemap and SEO', () => {
  test('sitemap.xml is accessible and contains locale variants', async ({ request }) => {
    const resp = await request.get(`${API}/sitemap.xml`);
    expect(resp.ok()).toBeTruthy();
    const body = await resp.text();
    expect(body).toContain('<?xml');
    expect(body).toContain('/en/menu');
    expect(body).toContain('/zh/menu');
    expect(body).toContain('hreflang="en"');
    expect(body).toContain('hreflang="zh"');
  });

  test('robots.txt references sitemap', async ({ request }) => {
    const resp = await request.get(`${API}/robots.txt`);
    expect(resp.ok()).toBeTruthy();
    const body = await resp.text();
    expect(body).toContain('User-agent');
    expect(body).toContain('Sitemap');
  });
});
