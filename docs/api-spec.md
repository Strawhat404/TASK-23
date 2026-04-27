# BrewFlow — API Specification

Base URL: `http://localhost:8000`

All responses follow the envelope: `{ "success": <bool>, "data": <T|null>, "error": <string|null> }`.

Authentication: session cookie `brewflow_session` (HMAC-signed). All protected endpoints require this cookie. There is no JWT bearer fallback — cookie-only authentication is enforced at the guard level.

---

## Auth

Mounted at `/api/auth`

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/auth/login` | — | Login with username/password; sets `brewflow_session` cookie and returns `{ session_cookie, user }` |
| POST | `/api/auth/register` | — | Register new customer account (assigned Customer role) |
| POST | `/api/auth/logout` | ✓ | Invalidate session and clear cookie |
| GET | `/api/auth/me` | ✓ | Get current user profile |
| PUT | `/api/auth/locale` | ✓ | Update preferred locale for the authenticated user |

---

## Products

Mounted at `/api/products`

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/products?featured=&limit=` | — | List active products with optional filters |
| GET | `/api/products/:id` | — | Get product detail including option groups |

---

## Cart

Mounted at `/api/cart`

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/cart/` | Customer | Get current cart with subtotal, tax, and total |
| POST | `/api/cart/add` | Customer | Add item to cart (validates options, required groups, SKU) |
| PUT | `/api/cart/:item_id` | Customer | Update cart item quantity |
| DELETE | `/api/cart/:item_id` | Customer | Remove a specific cart item |
| DELETE | `/api/cart/clear` | Customer | Clear all items from cart |

---

## Store & Pickup Slots

Mounted at `/api/store`

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/store/hours` | — | Get store opening hours by day of week |
| GET | `/api/store/pickup-slots?date=&prep_time=` | Optional | Get available pickup slots for a date; prep time derived from cart if authenticated |
| GET | `/api/store/tax` | — | Get active sales tax configuration |

---

## Orders

Mounted at `/api/orders`

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/orders/checkout` | Customer | Place order: acquires reservation locks, creates reservation + order atomically, returns voucher code |
| GET | `/api/orders/` | Customer | List own orders |
| GET | `/api/orders/:id` | Customer | Get order detail with items, fulfilment history, and reservation |
| POST | `/api/orders/:id/confirm` | Customer | Confirm a held reservation (must be in Held status and within hold window) |
| POST | `/api/orders/:id/cancel` | Customer/Admin | Cancel order (role-gated by fulfilment state machine) |

---

## Staff / Fulfilment

Mounted at `/api/staff`

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/staff/orders?status=` | Staff | List all orders, optionally filtered by status |
| GET | `/api/staff/orders/:id` | Staff | Get full order detail |
| PUT | `/api/staff/orders/:id/status` | Staff | Transition order status (validated by fulfilment state machine) |
| POST | `/api/staff/scan` | Staff | Scan a pickup voucher code; validates order state and flags mismatches |
| GET | `/api/staff/dashboard` | Staff | Dashboard order counts (Pending / InPrep / Ready) |
| GET | `/api/staff/dashboard/counts` | Staff | Alias for `/api/staff/dashboard` |

---

## Exam

Mounted at `/api/exam`

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/exam/subjects` | — | List exam subjects |
| GET | `/api/exam/subjects/:id/chapters` | — | List chapters for a subject |
| GET | `/api/exam/questions?subject_id=&chapter_id=&difficulty=&q=&page=&per_page=` | Teacher | Paginated question list with filters |
| GET | `/api/exam/questions/:id` | Teacher | Get question detail with options |
| POST | `/api/exam/import` | Teacher | Bulk import questions from CSV content |
| POST | `/api/exam/questions/import` | Teacher | Alias for `/api/exam/import` |
| POST | `/api/exam/generate` | Teacher | Generate a new exam version from question pool |
| GET | `/api/exam/versions` | ✓ | List all exam versions |
| GET | `/api/exam/versions/:id` | ✓ | Get exam version detail |

---

## Training

Mounted at `/api/training`

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/training/start/:exam_id` | ✓ | Start an exam attempt; returns questions and time limit |
| POST | `/api/training/answer` | ✓ | Submit an answer for a question in an attempt (or review mode if no attempt_id) |
| POST | `/api/training/finish/:attempt_id` | ✓ | Finish an attempt; computes score and returns wrong question detail |
| GET | `/api/training/attempts` | ✓ | List own exam attempts |
| GET | `/api/training/attempts/:id` | ✓ | Get attempt detail with per-question answers |
| GET | `/api/training/analytics` | ✓ | Score analytics: overall, by subject, by difficulty, recent attempts |
| POST | `/api/training/favorites/:question_id` | ✓ | Add question to favourites |
| DELETE | `/api/training/favorites/:question_id` | ✓ | Remove question from favourites |
| GET | `/api/training/favorites` | ✓ | List favourite questions |
| GET | `/api/training/wrong-notebook` | ✓ | Get wrong-answer notebook (all tracked wrong questions) |
| GET | `/api/training/review-session` | ✓ | Get a review session of questions due for re-practice |

---

## Dispatch

Mounted at `/api/dispatch`

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/dispatch/zones` | Staff | List station zones |
| GET | `/api/dispatch/queue?zone_id=` | Staff | Get queued grab tasks (limited by config) |
| POST | `/api/dispatch/grab/:task_id` | Staff | Grab an unassigned task |
| POST | `/api/dispatch/accept/:task_id` | Staff | Accept an offered task |
| POST | `/api/dispatch/reject/:task_id` | Staff | Reject an offered task |
| POST | `/api/dispatch/start/:task_id` | Staff | Start an assigned task (ownership verified) |
| POST | `/api/dispatch/complete/:task_id` | Staff | Complete a task and record score |
| GET | `/api/dispatch/my-tasks` | Staff | List active tasks assigned to the authenticated staff member |
| POST | `/api/dispatch/assign` | Admin | Assign or enqueue a task for an order |
| GET | `/api/dispatch/recommendations/:order_id` | Admin | Get staff recommendations for an order |
| GET | `/api/dispatch/shifts?user_id=&date=` | Staff | Get shifts for a staff member on a date |
| POST | `/api/dispatch/shifts` | Admin | Create a shift window |
| GET | `/api/dispatch/reputation/:user_id` | Admin | Get staff reputation record |

---

## Admin

Mounted at `/api/admin`

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/admin/users` | Admin | List all users with roles |
| POST | `/api/admin/users/:id/roles` | Admin | Assign a role to a user |
| DELETE | `/api/admin/users/:id/roles/:role` | Admin | Remove a role from a user |
| PUT | `/api/admin/store-hours` | Admin | Update store opening hours |
| PUT | `/api/admin/tax` | Admin | Update sales tax configuration |
| POST | `/api/admin/products` | Admin | Create a new product (SPU) |
| PUT | `/api/admin/products/:id` | Admin | Update an existing product |

---

## i18n

Mounted at `/api/i18n`

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/i18n/translations/:locale` | — | Get translation map for a locale (`en`, `zh`) |
| GET | `/api/i18n/locales` | — | List supported locales |

---

## Health

Mounted at `/health`

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/health/` | — | Basic health check; 503 if DB unreachable |
| GET | `/health/detailed` | Admin | Full health report including background job statuses and degradation state |
| GET | `/health/ready` | — | Readiness probe; 503 if DB or critical services are degraded |
| GET | `/health/live` | — | Liveness probe; always 200 if process is alive |

---

## Sitemap / SEO

Mounted at `/`

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/sitemap.xml` | — | XML sitemap with EN/ZH hreflang alternates |
| GET | `/robots.txt` | — | Robots file pointing to sitemap |
