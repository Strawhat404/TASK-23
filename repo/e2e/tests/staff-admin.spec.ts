import { test, expect } from '@playwright/test';

const API = process.env.API_URL || 'http://localhost:8000';

async function loginApi(request: any, username: string, password: string): Promise<string> {
  const resp = await request.post(`${API}/api/auth/login`, {
    data: { username, password },
  });
  const body = await resp.json();
  return body.data.session_cookie;
}

test.describe('Staff dashboard', () => {
  test('staff can access dashboard counts', async ({ request }) => {
    const cookie = await loginApi(request, 'staff', 'StaffPass123!');
    const resp = await request.get(`${API}/api/staff/dashboard/counts`, {
      headers: { Cookie: `brewflow_session=${cookie}` },
    });
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(body.data.pending_count).toBeGreaterThanOrEqual(0);
    expect(body.data.in_prep_count).toBeGreaterThanOrEqual(0);
    expect(body.data.ready_count).toBeGreaterThanOrEqual(0);
  });

  test('staff can list orders', async ({ request }) => {
    const cookie = await loginApi(request, 'staff', 'StaffPass123!');
    const resp = await request.get(`${API}/api/staff/orders`, {
      headers: { Cookie: `brewflow_session=${cookie}` },
    });
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(body.success).toBe(true);
    expect(Array.isArray(body.data)).toBe(true);
  });

  test('customer cannot access staff dashboard', async ({ request }) => {
    const cookie = await loginApi(request, 'customer', 'CustomerPass123!');
    const resp = await request.get(`${API}/api/staff/orders`, {
      headers: { Cookie: `brewflow_session=${cookie}` },
    });
    expect(resp.status()).toBe(403);
  });
});

test.describe('Admin panel', () => {
  test('admin can list users', async ({ request }) => {
    const cookie = await loginApi(request, 'admin', 'AdminPass123!');
    const resp = await request.get(`${API}/api/admin/users`, {
      headers: { Cookie: `brewflow_session=${cookie}` },
    });
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(body.success).toBe(true);
    expect(Array.isArray(body.data)).toBe(true);
    expect(body.data.length).toBeGreaterThan(0);
    // Verify user shape
    const first = body.data[0];
    expect(first.username).toBeTruthy();
    expect(Array.isArray(first.roles)).toBe(true);
  });

  test('staff cannot access admin panel', async ({ request }) => {
    const cookie = await loginApi(request, 'staff', 'StaffPass123!');
    const resp = await request.get(`${API}/api/admin/users`, {
      headers: { Cookie: `brewflow_session=${cookie}` },
    });
    expect(resp.status()).toBe(403);
  });

  test('admin can view health report', async ({ request }) => {
    const cookie = await loginApi(request, 'admin', 'AdminPass123!');
    const resp = await request.get(`${API}/health/detailed`, {
      headers: { Cookie: `brewflow_session=${cookie}` },
    });
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(body.status).toBeTruthy();
    expect(body.database).toBeTruthy();
    expect(Array.isArray(body.services)).toBe(true);
  });
});
