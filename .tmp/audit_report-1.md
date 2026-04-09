# Delivery Acceptance and Project Architecture Audit (Static-Only Re-Run #2)

## 1. Verdict
- Overall conclusion: **Partial Pass**
- Reason:
  - Prior blockers around checkout slot payload shape and voucher hash column-length mismatch appear fixed.
  - Material High/Medium issues remain in security policy fit, concurrency safety, validation, UI status logic, and test coverage depth.

## 2. Scope and Static Verification Boundary
- What was reviewed:
  - Backend routes/services/db/middleware, frontend pages/state/components, shared DTOs, SQL migrations, and README/scripts/manifests.
- What was not reviewed:
  - Runtime behavior, browser execution, DB runtime modes, deployment behavior.
- What was intentionally not executed:
  - Project run/start, tests, Docker, external integrations.
- Claims requiring manual verification:
  - Actual browser cookie behavior (`Secure`, SameSite, rotation on real requests).
  - Runtime migration application outcomes across environments.
  - Runtime UI render/interaction quality.

## 3. Repository / Requirement Mapping Summary
- Prompt core goal mapped to implementation areas:
  - Retail ordering + reservations/vouchers/fulfillment: `cart`, `store`, `orders`, `staff`.
  - Training + question bank/exams/favorites/notebook: `exam`, `training`, `admin`.
  - Auth/session/security: `auth`, `auth_guard`, `session`, `sessions` migration.
  - Fault-tolerance/health/degradation/dispatch: `resilience`, `health`, `dispatch`.
- Re-run #2 delta:
  - Fixed from prior reports:
    - Pickup slots API contract now aligned: backend returns `ApiResponse<Vec<PickupSlot>>`.
    - Voucher hash width fix migration added (`010_fix_voucher_hash_column_lengths.sql`).
    - Prep-time now derived from cart max prep time for slot listing + checkout validation.
    - Reservation lock quantity handling updated to quantity-aware decrements.

## 4. Section-by-section Review

### 4.1 Hard Gates

#### 4.1.1 Documentation and static verifiability
- Conclusion: **Pass**
- Rationale:
  - README now provides startup/test/config instructions and migration order.
- Evidence:
  - `README.md:5`, `README.md:16`, `README.md:33`, `README.md:42`, `README.md:53`, `README.md:69`
  - `run_tests.sh:1`
- Manual verification note:
  - README contains one route inconsistency (`/api/health` vs mounted `/health`) requiring correction.

#### 4.1.2 Material deviation from prompt
- Conclusion: **Partial Pass**
- Rationale:
  - Core business scope is implemented; however strict cookie-session requirement is diluted by bearer fallback and token-based frontend flow.
- Evidence:
  - Cookie + bearer fallback in guard: `backend/src/middleware/auth_guard.rs:12`, `backend/src/middleware/auth_guard.rs:28`
  - JWT still issued to frontend and stored in app state: `backend/src/routes/auth.rs:68`, `frontend/src/pages/auth.rs:78`, `frontend/src/state/mod.rs:63`

### 4.2 Delivery Completeness

#### 4.2.1 Core explicit requirements coverage
- Conclusion: **Partial Pass**
- Rationale:
  - Coverage is broad and improved, including fixed slot/voucher issues.
  - Remaining gaps: strict auth policy fit, dispatch race resilience, input validation gaps.
- Evidence:
  - Slot alignment fixed: `frontend/src/pages/checkout.rs:46`, `backend/src/routes/store.rs:34`, `backend/src/routes/store.rs:74`
  - Voucher length migration: `database/migrations/010_fix_voucher_hash_column_lengths.sql:10`
  - Remaining auth deviation: `backend/src/middleware/auth_guard.rs:28`

#### 4.2.2 End-to-end 0→1 deliverable
- Conclusion: **Partial Pass**
- Rationale:
  - Structure, migrations, scripts, and docs are present.
  - Missing integration tests and unresolved high risks prevent full acceptance.
- Evidence:
  - Workspace/services structure: `Cargo.toml:1`, `backend/src/main.rs:151`
  - Tests limited to service-unit modules: `backend/src/services/pricing.rs:46`, `backend/src/services/fulfillment.rs:71`, `backend/src/services/session.rs:75`, `backend/src/services/crypto.rs:103`

### 4.3 Engineering and Architecture Quality

#### 4.3.1 Structure and module decomposition
- Conclusion: **Pass**
- Rationale:
  - Clear decomposition by route/service/db and frontend pages/state; migrations segmented.
- Evidence:
  - `backend/src/main.rs:151`
  - `backend/src/services/mod.rs:1`
  - `frontend/src/main.rs:11`

#### 4.3.2 Maintainability/extensibility
- Conclusion: **Partial Pass**
- Rationale:
  - Generally maintainable modular shape; however key hardcoded values and placeholder observability remain.
- Evidence:
  - Hardcoded sitemap base URL: `backend/src/routes/sitemap.rs:19`
  - Health job status placeholder: `backend/src/services/resilience.rs:362`

### 4.4 Engineering Details and Professionalism

#### 4.4.1 Error handling, logging, validation, API design
- Conclusion: **Partial Pass**
- Rationale:
  - Structured API responses and many guarded routes are present.
  - Validation remains incomplete for cart quantity/options; dispatch acceptance race lacks robust DB-level conflict handling.
- Evidence:
  - API wrapper: `shared/src/dto.rs:362`
  - Cart add/update no explicit quantity lower-bound validation: `backend/src/routes/cart.rs:99`, `backend/src/routes/cart.rs:206`
  - Dispatch accept update does not verify affected row ownership/state atomically: `backend/src/db/dispatch.rs:304`, `backend/src/services/dispatch.rs:178`

#### 4.4.2 Product/service maturity
- Conclusion: **Partial Pass**
- Rationale:
  - Product-like breadth exists; test and observability depth is still not product-grade.
- Evidence:
  - Health/readiness/liveness routes exist: `backend/src/routes/health.rs:19`, `backend/src/routes/health.rs:58`, `backend/src/routes/health.rs:93`
  - Detailed job status path returns empty list placeholder: `backend/src/services/resilience.rs:367`

### 4.5 Prompt Understanding and Requirement Fit

#### 4.5.1 Business and constraints fit
- Conclusion: **Partial Pass**
- Rationale:
  - Strong implementation alignment across ordering/training roles and workflows.
  - Still diverges from strict rotating-cookie-only session model in practical frontend/backend behavior.
- Evidence:
  - Role guards: `backend/src/middleware/auth_guard.rs:143`, `backend/src/middleware/auth_guard.rs:170`, `backend/src/middleware/auth_guard.rs:197`
  - Fulfillment cancel-after-ready admin gate: `backend/src/services/fulfillment.rs:24`
  - Bearer fallback and frontend token headers: `backend/src/middleware/auth_guard.rs:28`, `frontend/src/pages/checkout.rs:28`

### 4.6 Aesthetics (frontend/full-stack)

#### 4.6.1 Visual and interaction quality
- Conclusion: **Cannot Confirm Statistically**
- Rationale:
  - UI structure is present but runtime visual quality and interaction behavior need manual runtime check.
- Evidence:
  - `frontend/src/pages/checkout.rs:59`, `frontend/src/pages/staff.rs:76`, `frontend/src/components/status_badge.rs:1`

## 5. Issues / Suggestions (Severity-Rated)

1. Severity: **High**
- Title: Session model still allows bearer-token auth path, weakening strict rotating-cookie requirement
- Conclusion: **Fail**
- Evidence:
  - Guard fallback to bearer token: `backend/src/middleware/auth_guard.rs:28`
  - JWT issued at login and stored client-side: `backend/src/routes/auth.rs:68`, `frontend/src/pages/auth.rs:78`, `frontend/src/state/mod.rs:63`
- Impact:
  - Requirement fit risk: session behavior can bypass cookie-only control expectations.
- Minimum actionable fix:
  - Enforce cookie-only auth for browser-protected endpoints, or explicitly scope bearer tokens to non-browser/internal clients with matched idle semantics and policy boundaries.

2. Severity: **High**
- Title: Dispatch accept path remains race-prone for double acceptance under concurrency
- Conclusion: **Partial Fail**
- Evidence:
  - Acceptance uses update on broad status set without checking rows affected semantics in service layer: `backend/src/db/dispatch.rs:304`, `backend/src/services/dispatch.rs:178`
- Impact:
  - Two workers can race on same queued/offered task with ambiguous winner handling.
- Minimum actionable fix:
  - Use atomic conditional update with rows-affected validation and return explicit conflict when `0` rows updated; consider transaction/locking for offer/accept flow.

3. Severity: **Medium**
- Title: Cart quantity/options validation remains under-specified at route level
- Conclusion: **Partial Fail**
- Evidence:
  - `add_to_cart` and `update_item` accept quantity directly without explicit lower-bound validation: `backend/src/routes/cart.rs:99`, `backend/src/routes/cart.rs:206`
- Impact:
  - Invalid cart states can propagate into pricing/stock paths.
- Minimum actionable fix:
  - Validate quantity (`>=1`) and enforce required option-group constraints before DB writes.

4. Severity: **Medium**
- Title: Frontend order detail status checks still use lowercase values inconsistent with backend enums
- Conclusion: **Fail**
- Evidence:
  - Frontend checks lowercase statuses: `frontend/src/pages/orders.rs:164`, `frontend/src/pages/orders.rs:165`, `frontend/src/pages/orders.rs:214`
  - Backend statuses are title-case enum values: `database/migrations/004_cart_and_orders.sql:58`, `database/migrations/003_store_and_reservations.sql:22`
- Impact:
  - Confirm/cancel/timer affordances may render incorrectly.
- Minimum actionable fix:
  - Normalize status comparison via shared enums or case-insensitive mapping helpers.

5. Severity: **Medium**
- Title: Detailed health report omits background job statuses (stubbed)
- Conclusion: **Partial Fail**
- Evidence:
  - `get_job_statuses()` currently returns empty vector: `backend/src/services/resilience.rs:362`, `backend/src/services/resilience.rs:367`
- Impact:
  - Operational troubleshooting visibility is reduced.
- Minimum actionable fix:
  - Wire `BackgroundJobManager` state into `DegradationManager`/health report output.

6. Severity: **Low**
- Title: README health endpoint path inconsistent with mounted route
- Conclusion: **Fail**
- Evidence:
  - README says `http://localhost:8000/api/health`: `README.md:14`
  - Backend mounts health routes at `/health`: `backend/src/main.rs:162`, `backend/src/routes/health.rs:19`
- Impact:
  - Documentation confusion during manual verification.
- Minimum actionable fix:
  - Update README to correct endpoint examples.

## 6. Security Review Summary
- authentication entry points: **Partial Pass**
  - Evidence: local username/password + bcrypt + session cookie issuance implemented (`backend/src/routes/auth.rs:33`, `backend/src/routes/auth.rs:54`, `backend/src/routes/auth.rs:87`).
  - Gap: bearer fallback path remains (`backend/src/middleware/auth_guard.rs:28`).

- route-level authorization: **Pass**
  - Evidence: Staff/Admin/Teacher guards applied across sensitive routes (`backend/src/routes/staff.rs:29`, `backend/src/routes/admin.rs:70`, `backend/src/routes/exam.rs:49`, `backend/src/routes/dispatch.rs:141`).

- object-level authorization: **Partial Pass**
  - Evidence: ownership checks exist for orders/cart/training attempts (`backend/src/routes/orders.rs:457`, `backend/src/routes/cart.rs:195`, `backend/src/routes/training.rs:133`).
  - Gap: dispatch accept race can still permit unsafe state transitions.

- function-level authorization: **Pass**
  - Evidence: cancel-after-ready admin gate enforced (`backend/src/services/fulfillment.rs:24`, `backend/src/routes/orders.rs:519`).

- tenant / user isolation: **Partial Pass**
  - Evidence: user-scoped access checks present.
  - Gap: concurrency conflict robustness in dispatch remains insufficient.

- admin/internal/debug protection: **Pass**
  - Evidence: `AdminGuard` on `/health/detailed` and admin domains (`backend/src/routes/health.rs:46`, `backend/src/routes/admin.rs:70`).

## 7. Tests and Logging Review
- Unit tests: **Partial Pass**
  - Evidence: service-level tests only (`backend/src/services/pricing.rs:46`, `backend/src/services/session.rs:75`, `backend/src/services/fulfillment.rs:71`, `backend/src/services/crypto.rs:103`).

- API/integration tests: **Fail**
  - Evidence: no static signs of route-level integration tests (`rocket::local`, request/response integration harness) in source scan.

- Logging categories/observability: **Partial Pass**
  - Evidence: masking fairing + tracing usage exist (`backend/src/middleware/log_mask.rs:11`, `backend/src/routes/orders.rs:300`+ paths with tracing).
  - Gap: health job statuses are stubbed empty (`backend/src/services/resilience.rs:367`).

- Sensitive-data leakage risk in logs/responses: **Partial Pass**
  - Evidence: sensitive keys masked by fairing (`backend/src/middleware/log_mask.rs:11`).
  - Boundary: runtime verification required to confirm all response/log paths are masked consistently.

## 8. Test Coverage Assessment (Static Audit)

### 8.1 Test Overview
- Unit tests exist: **Yes** (service modules).
- API/integration tests exist: **Not found statically**.
- Frameworks: Rust built-in `#[cfg(test)]` / `#[test]`.
- Test entry points: inline module tests only.
- Documentation test commands: present (`README.md:16`, `README.md:69`, `run_tests.sh:1`).

### 8.2 Coverage Mapping Table

| Requirement / Risk Point | Mapped Test Case(s) | Key Assertion / Fixture / Mock | Coverage Assessment | Gap | Minimum Test Addition |
|---|---|---|---|---|---|
| Fulfillment transition rules incl. admin cancel gate | `backend/src/services/fulfillment.rs:81` | transition assertions incl Ready->Canceled admin gate | basically covered | no route/DB integration assertions | add route-level tests for cancel/status endpoints |
| Session cookie signing/rotation utility | `backend/src/services/session.rs:87` | sign/verify/rotation helper tests | basically covered | no login+guard lifecycle integration tests | add auth integration tests for cookie expiry/rotation/401/403 |
| Voucher crypto utility | `backend/src/services/crypto.rs:115` | encrypt/decrypt and failure-path tests | basically covered | no migration/data-path compatibility tests | add db-level integration tests for hashed+encrypted voucher flow |
| Checkout slot payload contract | none | N/A | missing | currently relies on code alignment only | add API schema/serialization contract tests frontend+backend |
| Dispatch concurrent accept safety | none | N/A | insufficient | race conditions can evade current tests | add concurrency tests asserting single successful accept |
| Cart quantity/options validation | none | N/A | missing | invalid quantities/options untested | add validation tests for negative/zero qty + invalid options |

### 8.3 Security Coverage Audit
- authentication: **insufficient** (no end-to-end auth integration tests).
- route authorization: **insufficient** (no comprehensive 401/403 route matrix tests).
- object-level authorization: **insufficient** (limited/no cross-user abuse tests).
- tenant/data isolation: **insufficient** (no concurrency isolation suite).
- admin/internal protection: **insufficient** (guards present, not integration-tested).

### 8.4 Final Coverage Judgment
- **Fail**
- Boundary:
  - Major pure-logic services are unit-tested.
  - Core API security and concurrency risks remain largely untested; severe defects could still pass current tests.

## 9. Final Notes
- Static-only audit boundary respected: no runtime claims were made.
- Significant improvements from prior iteration are visible and credited.
- Remaining High/Medium issues are root-cause focused and evidence-traceable.
