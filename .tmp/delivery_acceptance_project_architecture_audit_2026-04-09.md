# Delivery Acceptance and Project Architecture Audit (Static-Only)

## 1. Verdict
- Overall conclusion: **Fail**
- Primary reason: a **Blocker** security defect allows public self-registration into privileged roles (`Admin`, `Staff`, etc.), undermining authorization boundaries.

## 2. Scope and Static Verification Boundary
- Reviewed scope:
  - Documentation/config: `repo/README.md`, `repo/docs/api-spec.md`, `docs/api-spec.md`, `docs/design.md`, `docs/questions.md`, `repo/.env.example`, `repo/Rocket.toml`, `repo/.github/workflows/ci.yml`, `repo/run_tests.sh`
  - Backend entry/routes/middleware/services/db/migrations under `repo/backend/src/**` and `repo/database/migrations/**`
  - Frontend pages/components/routes under `repo/frontend/src/**`
  - Static test artifacts: inline `#[cfg(test)]` modules + `repo/backend/src/api_tests.rs`
- Not reviewed:
  - Runtime behavior in browser/server/DB containers
  - External integrations and network behavior
- Intentionally not executed:
  - Project startup, Docker, tests, migrations, or any runtime workflow
- Claims requiring manual verification:
  - End-to-end timing correctness across environments (hold timer, slot availability under timezone settings)
  - UI rendering quality/interaction in real browser
  - Concurrency behavior under production-like contention loads

## 3. Repository / Requirement Mapping Summary
- Prompt core goal mapped: offline retail pickup ordering + internal barista training in one UI, with SPU/SKU options, tax/price visibility, hold+voucher flow, staff fulfillment state machine, training exam lifecycle, localization, and operational resilience.
- Main implementation areas mapped:
  - Retail/order flow: `repo/backend/src/routes/{products,cart,orders,store,staff}.rs`, `repo/backend/src/services/{pickup,pricing,fulfillment,reservation_lock}.rs`, `repo/database/migrations/002..004,007,008.sql`
  - Training flow: `repo/backend/src/routes/{exam,training}.rs`, `repo/backend/src/db/{exam,training,analytics}.rs`, `repo/database/migrations/005.sql`
  - Auth/security: `repo/backend/src/routes/auth.rs`, `repo/backend/src/middleware/auth_guard.rs`, `repo/backend/src/services/{auth,session,crypto}.rs`
  - Frontend routing/i18n: `repo/frontend/src/main.rs`, `repo/frontend/src/components/locale_switcher.rs`, `repo/backend/src/routes/{i18n,sitemap}.rs`

## 4. Section-by-section Review

### 4.1 Hard Gates

#### 4.1.1 Documentation and static verifiability
- Conclusion: **Partial Pass**
- Rationale:
  - Startup/test/config docs exist and are mostly usable.
  - But documentation is inconsistent across locations (JWT fallback claim vs cookie-only implementation; root API spec diverges from real routes), increasing verification friction.
- Evidence:
  - `repo/README.md:5`, `repo/README.md:16`, `repo/README.md:33`, `repo/README.md:97`
  - Cookie-only auth in code: `repo/backend/src/middleware/auth_guard.rs:14-16`
  - Contradictory JWT fallback claims: `docs/design.md:64`, `docs/questions.md:15-16`
  - Root API spec path mismatch examples: `docs/api-spec.md:38-41` vs actual cart routes `repo/backend/src/routes/cart.rs:98`, `repo/backend/src/routes/cart.rs:256`, `repo/backend/src/routes/cart.rs:369`
- Manual verification note: N/A (pure static consistency issue)

#### 4.1.2 Material deviation from Prompt
- Conclusion: **Partial Pass**
- Rationale:
  - Project is centered on the requested domain and technologies.
  - Material gaps/defects remain in security boundaries and some requirement semantics (localization/time handling, resilience wiring, analytics correctness).
- Evidence:
  - Core scope present: route mounts `repo/backend/src/main.rs:162-174`
  - Security boundary break (role escalation): `repo/backend/src/routes/auth.rs:165-168`, `repo/frontend/src/pages/auth.rs:229-237`

### 4.2 Delivery Completeness

#### 4.2.1 Core requirements coverage
- Conclusion: **Partial Pass**
- Rationale:
  - Implemented: SPU/SKU browsing, required options validation, tax/pricing breakdown, 15-min slot generation with prep-time blocking, voucher+hold flow, fulfillment transitions, training modules, localization routes, sitemap.
  - Gaps: localized date/time rendering in UI is largely raw strings; mismatch warning flow is only partially wired from UI; anti-crawling/import resilience appears mostly scaffolded.
- Evidence:
  - Required options enforced: `repo/backend/src/routes/cart.rs:98-114` and comment in spec `repo/docs/api-spec.md:75-76`
  - 15-minute slot logic + prep block: `repo/backend/src/services/pickup.rs:6-9`, `repo/backend/src/services/pickup.rs:43-46`, `repo/backend/src/services/pickup.rs:65-71`
  - Hold default 10 minutes: `repo/backend/src/routes/orders.rs:194`
  - Fulfillment state machine + cancel-after-ready admin rule: `repo/backend/src/services/fulfillment.rs:15-27`
  - Localization routes and locale-prefixed URLs: `repo/backend/src/routes/i18n.rs:23-73`, `repo/frontend/src/main.rs:25-79`
  - Raw date strings in UI (not localized formatting): `repo/frontend/src/pages/orders.rs:59`, `repo/frontend/src/pages/orders.rs:167`, `repo/frontend/src/pages/orders.rs:251`
  - Staff mismatch input not sent: `repo/backend/src/routes/staff.rs:256-259` vs `repo/frontend/src/pages/staff.rs:297`
  - Anti-crawling config not wired broadly: `repo/backend/src/services/resilience.rs:375-452`; usage search only hits same file `repo/backend/src/services/resilience.rs:372`, `repo/backend/src/services/resilience.rs:375`, `repo/backend/src/services/resilience.rs:389`
- Manual verification note:
  - End-to-end localization UX and mismatch flows need manual UI/API walkthrough.

#### 4.2.2 End-to-end 0→1 deliverable vs partial/demo
- Conclusion: **Pass**
- Rationale:
  - Multi-module backend/frontend/shared/database structure is complete and product-like, not a single-file demo.
- Evidence:
  - Structure documented: `repo/README.md:97-104`
  - Entry points and route mounts complete: `repo/backend/src/main.rs:152-175`, `repo/frontend/src/main.rs:19-79`

### 4.3 Engineering and Architecture Quality

#### 4.3.1 Structure and decomposition reasonableness
- Conclusion: **Pass**
- Rationale:
  - Codebase is decomposed by domain (routes/db/services/middleware), with explicit route guards and shared DTO/model layers.
- Evidence:
  - Backend modular layout used in entrypoint: `repo/backend/src/main.rs:1-4`, `repo/backend/src/main.rs:162-174`
  - Shared DTO/model usage in routes: `repo/backend/src/routes/staff.rs:7-12`

#### 4.3.2 Maintainability/extensibility
- Conclusion: **Partial Pass**
- Rationale:
  - Overall maintainable structure exists.
  - But dead/unused resilience paths and inconsistent docs indicate drift risk.
- Evidence:
  - Degradation-aware scheduler method exists: `repo/backend/src/services/resilience.rs:505-538`
  - Runtime loop uses `get_due_jobs` instead: `repo/backend/src/main.rs:90-92`
  - `get_due_jobs` ignores degradation state: `repo/backend/src/services/resilience.rs:618-638`

### 4.4 Engineering Details and Professionalism

#### 4.4.1 Error handling, logging, validation, API design
- Conclusion: **Partial Pass**
- Rationale:
  - Positive: many explicit status codes and validations, masked response logging, role guards.
  - Negative: blocker auth flaw; multiple correctness bugs (timezone mismatch, analytics status mismatch).
- Evidence:
  - Password policy validation present: `repo/backend/src/services/auth.rs:22-49`, enforced in register `repo/backend/src/routes/auth.rs:106-107`
  - Masked response logging: `repo/backend/src/middleware/log_mask.rs:11-19`, `repo/backend/src/middleware/log_mask.rs:46-50`
  - Timezone inconsistency: `repo/backend/src/routes/orders.rs:194`, `repo/backend/src/routes/orders.rs:497`, `repo/frontend/src/components/hold_timer.rs:67`
  - Analytics status mismatch: `repo/backend/src/db/training.rs:51`, `repo/backend/src/db/analytics.rs:37`, schema enum `repo/database/migrations/005_exam_system.sql:124`

#### 4.4.2 Product/service shape vs demo shape
- Conclusion: **Partial Pass**
- Rationale:
  - Product-like scope exists, but significant defects in security and some core behavior prevent acceptance as production-ready delivery.
- Evidence:
  - Full modules/features present: `repo/backend/src/main.rs:162-174`
  - Blocker auth defect: `repo/backend/src/routes/auth.rs:165-168`

### 4.5 Prompt Understanding and Requirement Fit

#### 4.5.1 Business-goal and constraint fit
- Conclusion: **Partial Pass**
- Rationale:
  - Broad feature intent is understood and implemented.
  - Several key constraints are weakened: authorization boundary, localized date/time behavior, resilience/degradation behavior fidelity.
- Evidence:
  - Dispatch scoring factors exist (zone/time/workload/reputation): `repo/backend/src/services/dispatch.rs:45-47`, `repo/backend/src/services/dispatch.rs:133`
  - Queue controls for concurrent grab/accept are designed: `repo/backend/src/services/dispatch.rs:192-194`, `repo/backend/src/services/dispatch.rs:225-226`
  - Degradation scheduling mismatch: `repo/backend/src/services/resilience.rs:505-538` vs `repo/backend/src/main.rs:90-92` and `repo/backend/src/services/resilience.rs:618-638`

### 4.6 Aesthetics (frontend)

#### 4.6.1 Visual/interaction quality fit
- Conclusion: **Cannot Confirm Statistically**
- Rationale:
  - Static code shows structured pages/components and consistent utility-class usage, but final rendering quality and interaction states cannot be proven without runtime/browser validation.
- Evidence:
  - UI composition examples: `repo/frontend/src/pages/orders.rs:52-67`, `repo/frontend/src/pages/staff.rs:286-315`
  - Locale switcher exists: `repo/frontend/src/components/locale_switcher.rs:24-47`
- Manual verification note:
  - Manual browser review required for spacing/alignment/hover states/visual regressions.

## 5. Issues / Suggestions (Severity-Rated)

### Blocker

1. **Severity: Blocker**
- Title: Public registration allows self-assignment of privileged roles
- Conclusion: **Fail**
- Evidence: `repo/backend/src/routes/auth.rs:21`, `repo/backend/src/routes/auth.rs:165-168`, `repo/frontend/src/pages/auth.rs:229-237`
- Impact: Any unauthenticated user can become `Admin`/`Staff`/`Teacher`, collapsing authorization and administrative trust boundaries.
- Minimum actionable fix: Remove `role` from public register payload, always assign `Customer` server-side, and keep role assignment only behind `AdminGuard` endpoints.

### High

2. **Severity: High**
- Title: Hold-expiry logic mixes local time and UTC
- Conclusion: **Fail**
- Evidence: `repo/backend/src/routes/orders.rs:194`, `repo/backend/src/routes/orders.rs:497`, `repo/frontend/src/components/hold_timer.rs:67`
- Impact: Holds may appear expired too early/late depending on timezone; can break reservation integrity and customer UX.
- Minimum actionable fix: Use UTC consistently for persistence/comparison/serialization, and format for locale only at presentation layer.

3. **Severity: High**
- Title: Training analytics filter status does not match stored enum values
- Conclusion: **Fail**
- Evidence: `repo/backend/src/db/training.rs:51-52`, `repo/backend/src/db/analytics.rs:37`, `repo/backend/src/db/analytics.rs:65`, `repo/backend/src/db/analytics.rs:91`, `repo/database/migrations/005_exam_system.sql:124`
- Impact: Analytics can silently undercount or return zero despite completed attempts.
- Minimum actionable fix: Standardize status constants and use exact enum literals (`Completed`) in all queries.

4. **Severity: High**
- Title: Degradation-aware scheduling path is not used by runtime loop
- Conclusion: **Fail**
- Evidence: `repo/backend/src/services/resilience.rs:505-538`, `repo/backend/src/main.rs:90-92`, `repo/backend/src/services/resilience.rs:618-638`
- Impact: Background job execution can ignore degradation availability checks, conflicting with requirement to degrade non-critical jobs while preserving core integrity.
- Minimum actionable fix: Route scheduling through `should_run` (or embed the same checks in `get_due_jobs`) and enforce critical/non-critical policies explicitly.

### Medium

5. **Severity: Medium**
- Title: Critical/non-critical job distinction is stored but not enforced when auto-disabling
- Conclusion: **Partial Fail**
- Evidence: `repo/backend/src/services/resilience.rs:460-467`, `repo/backend/src/services/resilience.rs:565-572`
- Impact: Critical jobs may be disabled after repeated failures, risking ordering/reservation integrity.
- Minimum actionable fix: Prevent auto-disable of critical jobs and use escalation/alerting instead.

6. **Severity: Medium**
- Title: Exam import writes enum values that may not match schema literals
- Conclusion: **Suspected Risk**
- Evidence: `repo/backend/src/routes/exam.rs:160-163`, `repo/backend/src/routes/exam.rs:170`, `repo/backend/src/routes/exam.rs:175`, `repo/database/migrations/005_exam_system.sql:33`, `repo/database/migrations/005_exam_system.sql:38`
- Impact: Insert failures or coercion-dependent behavior on question import.
- Minimum actionable fix: Normalize import values to exact DB enum literals (or migrate schema to consistent canonical values).

7. **Severity: Medium**
- Title: Store-hours weekday mapping in Home UI conflicts with schema convention
- Conclusion: **Fail**
- Evidence: `repo/database/migrations/003_store_and_reservations.sql:7`, `repo/frontend/src/pages/home.rs:119-127`
- Impact: Sunday may render incorrectly (`?`) and day labels can drift from actual schedule data.
- Minimum actionable fix: Align frontend mapping to 0..6 (`Sun..Sat`) and centralize mapping helper.

8. **Severity: Medium**
- Title: Voucher mismatch warning path is only partially wired from staff UI
- Conclusion: **Partial Fail**
- Evidence: `repo/backend/src/routes/staff.rs:256-259`, `repo/frontend/src/pages/staff.rs:297`
- Impact: “Wrong order presented” warning logic may not trigger from default UI flow.
- Minimum actionable fix: Include presented `order_id` in scan request when staff is validating against a selected order context.

9. **Severity: Medium**
- Title: Product list docs/frontend imply `featured`/`limit` filtering but backend ignores query params
- Conclusion: **Partial Fail**
- Evidence: `repo/docs/api-spec.md:62`, `repo/frontend/src/pages/home.rs:16`, `repo/backend/src/routes/products.rs:9-13`
- Impact: Featured-products UI behavior is non-deterministic relative to specification.
- Minimum actionable fix: Implement query params in route/DB layer or remove unsupported API contract/documentation.

10. **Severity: Medium**
- Title: Date/time localization requirement is only partially met in frontend rendering
- Conclusion: **Partial Fail**
- Evidence: `repo/frontend/src/pages/orders.rs:59`, `repo/frontend/src/pages/orders.rs:167`, `repo/frontend/src/pages/orders.rs:251`
- Impact: Localized language exists, but timestamps/slots are displayed as raw backend strings rather than locale-aware formats.
- Minimum actionable fix: Add locale-aware date/time formatting utility and apply consistently across order/staff/training pages.

11. **Severity: Medium**
- Title: Documentation set contains conflicting auth/API contracts
- Conclusion: **Partial Fail**
- Evidence: `docs/design.md:64`, `docs/questions.md:15-16`, `repo/backend/src/middleware/auth_guard.rs:14-16`, `docs/api-spec.md:38-41`, `repo/backend/src/routes/cart.rs:98`
- Impact: Reviewers/maintainers can validate against wrong contracts, increasing acceptance and maintenance risk.
- Minimum actionable fix: Consolidate to one canonical spec (prefer `repo/docs/*`) and update/remove stale root docs.

## 6. Security Review Summary

- Authentication entry points: **Partial Pass**
  - Evidence: cookie signing/verification and idle timeout/rotation `repo/backend/src/services/session.rs:44-73`, `repo/backend/src/middleware/auth_guard.rs:56-77`.
  - Reasoning: mechanism is present and coherent, but registration privilege escalation critically weakens overall auth system trust.

- Route-level authorization: **Partial Pass**
  - Evidence: role guards in request guards `repo/backend/src/middleware/auth_guard.rs:120-197`; admin routes guarded `repo/backend/src/routes/admin.rs:67-71`.
  - Reasoning: guards exist broadly, but guard model can be bypassed by self-registering privileged roles.

- Object-level authorization: **Partial Pass**
  - Evidence: order ownership checks `repo/backend/src/routes/orders.rs:559-567`; attempt ownership checks `repo/backend/src/routes/training.rs:129-137`, `repo/backend/src/routes/training.rs:246-255`; dispatch task ownership `repo/backend/src/routes/dispatch.rs:101-103`, `repo/backend/src/routes/dispatch.rs:120-122`.
  - Reasoning: many object checks exist; no comprehensive proof for all object paths.

- Function-level authorization: **Fail**
  - Evidence: privileged role assignment in unauthenticated register flow `repo/backend/src/routes/auth.rs:165-168`.
  - Reasoning: function-level policy for role assignment is violated.

- Tenant / user isolation: **Partial Pass**
  - Evidence: user-scoped reads and ownership checks in cart/order/training paths `repo/backend/src/routes/cart.rs:274-283`, `repo/backend/src/routes/orders.rs:559-567`, `repo/backend/src/routes/training.rs:362-371`.
  - Reasoning: core isolation checks are present; still weakened by role-escalation defect.

- Admin / internal / debug protection: **Partial Pass**
  - Evidence: admin-protected health detail and admin APIs `repo/backend/src/routes/health.rs:112-116`, `repo/backend/src/routes/admin.rs:67-71`.
  - Reasoning: endpoints are guarded, but attacker can become admin via registration defect.

## 7. Tests and Logging Review

- Unit tests: **Partial Pass**
  - Evidence: inline unit-test modules exist for `auth`, `session`, `crypto`, `pricing`, `fulfillment`, `log_mask` (`repo/backend/src/services/auth.rs:58`, `repo/backend/src/services/session.rs:75`, `repo/backend/src/services/crypto.rs:103`, `repo/backend/src/services/fulfillment.rs:71`, `repo/backend/src/middleware/log_mask.rs:78`).
  - Reasoning: core utility logic has tests; business-critical security/authorization regression cases are missing.

- API / integration tests: **Partial Pass**
  - Evidence: `repo/backend/src/api_tests.rs` covers 401/403, login success/failure, required-options 422, expired hold 409, canceled voucher behavior (`repo/backend/src/api_tests.rs:155-189`, `repo/backend/src/api_tests.rs:232-255`, `repo/backend/src/api_tests.rs:258-299`, `repo/backend/src/api_tests.rs:301-337`).
  - Reasoning: good baseline, but no test coverage for registration role escalation, timezone consistency, analytics correctness, or dispatch race controls.

- Logging categories / observability: **Partial Pass**
  - Evidence: tracing init and structured logs in background jobs `repo/backend/src/main.rs:14`, `repo/backend/src/main.rs:96-114`; masked response-body logging `repo/backend/src/middleware/log_mask.rs:46-47`.
  - Reasoning: meaningful logs exist, but category strategy is basic and runtime behavior not executed here.

- Sensitive-data leakage risk in logs / responses: **Partial Pass**
  - Evidence: sensitive fields mask list includes `voucher_code`, `session_cookie`, `password` etc. `repo/backend/src/middleware/log_mask.rs:11-19`.
  - Reasoning: masking logic is present and tested (`repo/backend/src/middleware/log_mask.rs:87-120`), but static-only audit cannot prove all log callsites avoid sensitive fields.

## 8. Test Coverage Assessment (Static Audit)

### 8.1 Test Overview
- Unit tests exist via inline `#[cfg(test)]` in service/middleware modules.
- API/integration tests exist in `repo/backend/src/api_tests.rs` using Rocket local client.
- Frameworks/tooling: Rust `cargo test`, Rocket local client, `serde_json` assertions.
- Test entry points/docs:
  - README commands: `repo/README.md:16-21`, `repo/README.md:70-88`
  - CI runs both no-DB and DB test jobs: `repo/.github/workflows/ci.yml:14-39`, `repo/.github/workflows/ci.yml:40-92`
  - Dockerized local script exists: `repo/run_tests.sh:1-7`, `repo/run_tests.sh:102-104`

### 8.2 Coverage Mapping Table

| Requirement / Risk Point | Mapped Test Case(s) (`file:line`) | Key Assertion / Fixture / Mock (`file:line`) | Coverage Assessment | Gap | Minimum Test Addition |
|---|---|---|---|---|---|
| Auth required → 401 | `repo/backend/src/api_tests.rs:155-174` | no-cookie requests assert `Status::Unauthorized` | sufficient | None for basic unauthenticated guard | Add 401 checks for more protected routes beyond stubs |
| Tampered session cookie rejected | `repo/backend/src/api_tests.rs:178-189`; `repo/backend/src/services/session.rs:97-105` | forged cookie should be unauthorized / verify fails | sufficient | None for tamper primitive | Add expiry/rotation boundary tests at route level |
| Route authorization (403 for wrong role) | `repo/backend/src/api_tests.rs:232-255` | customer denied staff/admin endpoints | basically covered | Limited route surface covered | Add teacher/admin path matrix tests |
| Required options validation at add-to-cart | `repo/backend/src/api_tests.rs:258-277` | missing required options returns 422 | basically covered | No positive-path assertion with valid options+price deltas | Add success test asserting computed unit price/tax breakdown |
| Expired hold returns conflict | `repo/backend/src/api_tests.rs:280-299` | fixture order 9000 confirm returns 409 | basically covered | No timezone consistency test | Add tests with explicit UTC/local timestamps and boundary seconds |
| Canceled-order voucher scan mismatch behavior | `repo/backend/src/api_tests.rs:301-337` | `valid=false`, `mismatch=true` assertions | basically covered | No explicit wrong-order-presented case with `order_id` payload | Add scan test with mismatched `order_id` and reason assertion |
| Ready→Canceled admin-only rule | `repo/backend/src/services/fulfillment.rs:97-102` | unit-level transition rule assertions | insufficient | No API-level authorization/status-code coverage | Add `/api/orders/<id>/cancel` tests for Staff vs Admin when status=Ready |
| Registration privilege escalation prevention | none | none | missing | Critical security scenario untested | Add test: register with `role=Admin` must still result in only `Customer` role |
| Object-level authorization across user-owned resources | none explicit for orders/cart attempts in integration suite | none | insufficient | Severe IDOR-like defects could survive | Add 403 tests for accessing/modifying another user's order/cart/attempt |
| Analytics correctness after exam completion | none | none | missing | Status-case mismatch currently undetected | Add integration test: finish attempt then analytics totals > 0 |
| Dispatch race / double acceptance prevention | none | none | missing | Queue-control regressions not covered | Add concurrent accept/grab tests asserting single winner |

### 8.3 Security Coverage Audit
- Authentication: **Basically covered** for no-cookie and tampered-cookie cases (`repo/backend/src/api_tests.rs:155-189`), but missing idle-timeout/rotation integration tests.
- Route authorization: **Basically covered** by two 403 checks (`repo/backend/src/api_tests.rs:232-255`), but breadth is limited.
- Object-level authorization: **Insufficient**; no direct integration tests proving cross-user resource access is blocked for orders/cart/training.
- Tenant / data isolation: **Insufficient**; user-isolation behavior exists in code but lacks focused test assertions.
- Admin / internal protection: **Insufficient** because no test prevents privileged role self-assignment at registration; severe defect can remain undetected.

### 8.4 Final Coverage Judgment
- **Fail**
- Boundary explanation:
  - Covered: core auth guard basics, some role checks, key validation/error statuses.
  - Uncovered high-risk areas: registration privilege escalation, object-level isolation matrix, timezone expiry correctness, analytics correctness, dispatch concurrency controls.
  - Result: tests could still pass while severe security and business-integrity defects remain.

## 9. Final Notes
- This report is strictly static and evidence-based; runtime claims were avoided.
- The most urgent acceptance blocker is the registration-role privilege escalation path.
- After fixing Blocker/High issues, re-run a focused security + integration test review before acceptance.
