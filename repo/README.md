# BrewFlow

> **Project type:** Fullstack (Rust backend + Rust/WASM frontend)

A full-stack Rust web application built with Rocket (backend), Dioxus (frontend), and MySQL. It powers an offline beverage shop with in-store pickup ordering, staff fulfillment, and a barista training/exam platform — all from a single bilingual (English/Chinese) UI.

## Quick Start

```bash
cd repo
docker-compose up --build
```

> Also works with the newer CLI form: `docker compose up --build`

- Frontend: http://localhost:8080
- Backend API: http://localhost:8000
- Health check: http://localhost:8000/health/live

## Demo Credentials

The system ships with seeded users for **every role**. Use these to log in immediately after startup:

| Role | Username | Password | Access |
|------|----------|----------|--------|
| Admin | `admin` | `AdminPass123!` | Full access: admin panel, staff dashboard, training, all customer features |
| Staff | `staff` | `StaffPass123!` | Staff dashboard, order fulfillment, voucher scanning, dispatch |
| Customer | `customer` | `CustomerPass123!` | Browse menu, cart, checkout, orders, training exams |
| Teacher | `teacher` | `TeacherPass123!` | Customer features + question bank import, exam generation |
| AcademicAffairs | `academic` | `AcademicPass123!` | Customer features + full training/exam management |

Log in at http://localhost:8080/en/login (or `/zh/login` for Chinese).

## Verify the System is Running

After `docker-compose up --build` completes, verify with these steps:

```bash
# 1. Health check — should return {"success":true,"data":"alive"}
curl http://localhost:8000/health/live

# 2. Login as admin — should return session_cookie and user info
curl -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"AdminPass123!"}'

# 3. List products — should return the seeded beverage menu
curl http://localhost:8000/api/products/

# 4. Check store hours
curl http://localhost:8000/api/store/hours

# 5. Verify the frontend loads (should return HTML)
curl -s http://localhost:8080 | head -5
```

All five checks should succeed. If the health check fails, wait 30 seconds for MySQL to finish initialising and retry.

## Architecture

```
                    Browser (localhost:8080)
                           |
                    +--------------+
                    | Dioxus WASM  |  Single-page app, locale-prefixed routes
                    | Frontend     |  Components: navbar, menu, cart, checkout,
                    +--------------+  staff dashboard, training, admin panel
                           |
                     HTTP REST API
                           |
                    +--------------+
                    | Rocket       |  Routes: /api/auth, /api/products, /api/cart,
                    | Backend      |  /api/orders, /api/staff, /api/exam,
                    | (localhost:  |  /api/training, /api/admin, /api/dispatch,
                    |  8000)       |  /api/store, /api/i18n, /health, /sitemap.xml
                    +--------------+
                           |
                    +--------------+
                    | MySQL 8      |  Users, roles, products (SPU/SKU/options),
                    | (localhost:  |  carts, orders, reservations, vouchers,
                    |  3306)       |  questions, exams, attempts, analytics,
                    +--------------+  sessions, dispatch tasks, shifts, reputation
```

**Request flow:** Browser loads the Dioxus WASM app via nginx (port 8080). The app makes REST calls to the Rocket backend (port 8000). Every authenticated request carries a `brewflow_session` cookie signed with HMAC-SHA256. The backend validates the cookie, looks up the session in MySQL, checks role-based guards (Customer/Staff/Admin/Teacher/AcademicAffairs), and returns JSON responses. Sessions idle-timeout after 30 minutes and rotate IDs every 5 minutes.

## Environment Variables

All variables have sensible defaults for Docker. No manual configuration needed.

| Variable | Default (dev) | Description |
|---|---|---|
| `DATABASE_URL` | `mysql://root:root@localhost/brewflow` | MySQL connection string |
| `COOKIE_SECRET` | `brewflow-dev-cookie-secret` | HMAC-SHA256 secret for session cookies |
| `ENCRYPTION_KEY` | `brewflow-dev-encryption-key` | AES-256-GCM key for voucher encryption at rest |
| `ALLOWED_ORIGINS` | `http://localhost:8080` | Comma-separated CORS origins |
| `SITEMAP_BASE_URL` | `http://localhost:8080` | Base URL for sitemap.xml generation |

## Running Tests

```bash
cd repo
./run_tests.sh
```

The Dockerised harness spins up MySQL 8, applies all migrations, seeds test fixtures, then runs both `shared` and `backend` packages. No local Rust toolchain or database required.

### Test Tiers

| Tier | Location | Needs DB? | What it covers |
|------|----------|-----------|----------------|
| Shared unit | `shared/src/*.rs` | No | DTOs, enums, i18n translations, model serde |
| Backend unit | `backend/src/services/*.rs`, `middleware/*.rs` | No | Pricing, auth/hashing, crypto, session, fulfillment, pickup slots, resilience/circuit-breaker, CSV import, log masking, dispatch, health |
| Repository | `backend/src/db/*.rs` | No | Struct contracts, serde round-trips, pure helpers (sha256_hex) |
| API integration | `backend/src/api_tests.rs` | Mixed | All 74 production HTTP endpoints tested against real Rocket + real DB |
| End-to-end | `backend/src/api_tests.rs` (`e2e_*`) | Yes | Cross-layer journeys: register/login/browse/cart, role matrix, voucher scan, session rotation |
| Frontend unit | `frontend/src/components/*.rs`, `frontend/src/pages/*.rs` | No | Inline `#[cfg(test)]` with `use super::*` in every component and page file |
| Frontend logic | `frontend/src/logic.rs`, `frontend/src/state/mod.rs` | No | Price formatting, hold-timer, slot parsing, role predicates, cart math, URL helpers |
| Frontend contract | `frontend/tests/*.rs` | No | Component render contracts, page DTO shapes, CSS class computation, i18n label resolution |
| Browser E2E | `e2e/tests/*.spec.ts` | Yes | Playwright browser automation: login flow, customer journey, staff/admin panels, i18n/sitemap |

### Browser E2E Tests (Playwright)

A full Playwright suite lives in `e2e/` and runs entirely inside Docker:

```bash
cd repo

# 1. Start the application
docker-compose up --build -d

# 2. Wait for readiness
until curl -sf http://localhost:8000/health/live > /dev/null; do sleep 2; done

# 3. Run browser E2E tests (Docker-contained — no local Node.js needed)
docker build -t brewflow-e2e e2e/
docker run --rm --network host \
  -e BASE_URL=http://localhost:8080 \
  -e API_URL=http://localhost:8000 \
  brewflow-e2e
```

The suite covers 4 spec files:
- `auth.spec.ts` — login page rendering, valid/invalid credentials, API health + login contracts
- `customer-journey.spec.ts` — menu listing, product detail, store hours, tax rates
- `staff-admin.spec.ts` — staff dashboard, order list, admin user management, health report, role enforcement
- `i18n-sitemap.spec.ts` — locale switching, translation APIs, sitemap XML, robots.txt

The Dockerised `run_tests.sh` also invokes E2E tests automatically (pass `--skip-e2e` to omit).

## Session and Auth Policy

- Authentication uses **rotating HMAC-signed session cookies** (`brewflow_session`).
- Sessions have a **30-minute idle timeout**; each request resets the timer.
- Session IDs rotate every **5 minutes** to prevent fixation.
- Passwords require 12+ characters with uppercase, lowercase, digit, and special character.
- Voucher codes are encrypted at rest with AES-256-GCM.
- Sensitive fields (passwords, tokens, voucher codes) are masked in response logs.

### Role Matrix

| Role | Customer pages | Staff dashboard | Admin panel | Training/Exams | Dispatch |
|------|---------------|-----------------|-------------|----------------|----------|
| Customer | Yes | No | No | View only | No |
| Staff | Yes | Yes | No | View only | Yes |
| Teacher | Yes | No | No | Full (import, generate) | No |
| AcademicAffairs | Yes | No | No | Full | No |
| Admin | Yes | Yes | Yes | Full | Yes |

## Project Structure

```
repo/
  backend/       Rocket HTTP server (routes, services, DB layer, middleware)
  frontend/      Dioxus WASM single-page app (components, pages, state)
  shared/        DTOs, models, enums, and i18n shared between FE and BE
  database/      SQL migration files (applied in numeric order by Docker)
  docker-compose.yml
  run_tests.sh   Dockerised test harness
```
