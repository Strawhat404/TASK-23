# Targeted Issue Re-Check (Static-Only, Latest3)

Date: 2026-04-07
Scope: Re-verify the 6 previously reported issues using static evidence only.

## Results
1. Auth still allows bearer fallback (strict cookie-session policy fit remains partial): **Fixed**
- Conclusion: Bearer fallback is removed; request guard authenticates via session cookie path only.
- Evidence:
  - `repo/backend/src/middleware/auth_guard.rs:12` (guard doc now states cookie-only auth path)
  - `repo/backend/src/middleware/auth_guard.rs:26` (guard calls `try_cookie_auth` only)
  - `repo/backend/src/middleware/auth_guard.rs:51` (reads `brewflow_session` cookie)
  - `repo/frontend/src/state/mod.rs:12` and `repo/frontend/src/state/mod.rs:14` (frontend stores session cookie, not bearer token)
  - Repository-wide search shows no `Bearer`/`Authorization` usage in app source: `rg -n "Bearer|Authorization" repo/backend/src repo/frontend/src` (no matches)

2. Dispatch accept flow remains race-prone: **Fixed**
- Conclusion: Accept/grab updates are guarded atomically in SQL with state constraints and `rows_affected()` checks.
- Evidence:
  - `repo/backend/src/db/dispatch.rs:310` to `repo/backend/src/db/dispatch.rs:315` (atomic offered-task accept with `WHERE ... status='Offered' AND assigned_to=?`)
  - `repo/backend/src/db/dispatch.rs:322` (0 rows => `RowNotFound`)
  - `repo/backend/src/db/dispatch.rs:332` to `repo/backend/src/db/dispatch.rs:337` (atomic grab with `WHERE status='Queued'`)
  - `repo/backend/src/db/dispatch.rs:343` (0 rows => `RowNotFound`)
  - `repo/backend/src/services/dispatch.rs:210` (service uses atomic accept helper)

3. Cart validation gaps (quantity/options): **Fixed**
- Conclusion: Quantity floor validation and option-to-product ownership validation are present.
- Evidence:
  - `repo/backend/src/routes/cart.rs:107` and `repo/backend/src/routes/cart.rs:113` (`quantity < 1` rejected on add)
  - `repo/backend/src/routes/cart.rs:240` and `repo/backend/src/routes/cart.rs:246` (`quantity < 1` rejected on update)
  - `repo/backend/src/routes/cart.rs:132` to `repo/backend/src/routes/cart.rs:143` (each selected option must belong to SPU)
  - `repo/backend/src/db/products.rs:239` (SPU ownership check helper)

4. Frontend order detail status casing mismatch: **Fixed**
- Conclusion: Order detail logic now compares title-case status strings consistent with backend values.
- Evidence:
  - `repo/frontend/src/pages/orders.rs:164` (`status == "Pending"`)
  - `repo/frontend/src/pages/orders.rs:165` (`"Pending" || "Accepted"`)
  - `repo/frontend/src/pages/orders.rs:214` (`reservation.status == "Held"`)

5. Health job status output still stubbed: **Fixed (for detailed health endpoint path)**
- Conclusion: Detailed health route uses `BackgroundJobManager` real job status output.
- Evidence:
  - `repo/backend/src/routes/health.rs:47` (injects `Arc<BackgroundJobManager>`)
  - `repo/backend/src/routes/health.rs:51` (uses `job_mgr.get_job_statuses().await`)
  - `repo/backend/src/services/resilience.rs:579` to `repo/backend/src/services/resilience.rs:599` (implemented real job status mapping)
- Note: `DegradationManager::get_job_statuses` remains a stub (`repo/backend/src/services/resilience.rs:362` to `repo/backend/src/services/resilience.rs:367`), but this is not the code path used by `GET /health/detailed`.

6. README health endpoint path inconsistent with mounted route: **Fixed**
- Conclusion: README and route mount are aligned on `/health`.
- Evidence:
  - `repo/README.md:14` (documents `/health`, explicitly not `/api/health`)
  - `repo/backend/src/main.rs:158` (mounts health routes at `/health`)

## Static Boundary
- Static-only re-check performed.
- Not executed: project runtime, tests, Docker, external services.
- Any runtime behavior confirmation remains Manual Verification Required.
