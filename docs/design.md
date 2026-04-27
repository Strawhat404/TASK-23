# BrewFlow — System Design

## Overview

BrewFlow is an offline-first retail ordering and internal training platform for a multi-location food and beverage business. It runs entirely on the local network with no external service dependencies.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Browser (WASM)                    │
│              Dioxus Frontend (port 8080)             │
└──────────────────────┬──────────────────────────────┘
                       │ HTTP/HTTPS
┌──────────────────────▼──────────────────────────────┐
│              Rocket Backend (port 8000)              │
│   Routes / Services / DB Layer / Middleware          │
└──────────────────────┬──────────────────────────────┘
                       │ sqlx
┌──────────────────────▼──────────────────────────────┐
│                   MySQL 8.x                          │
└─────────────────────────────────────────────────────┘
```

## User Roles

| Role | Capabilities |
|------|-------------|
| Customer | Browse catalogue, manage cart, place orders, confirm/cancel orders, track order status |
| Staff | View and transition order fulfilment states, scan pickup vouchers, manage dispatch tasks, access training and exams |
| Teacher | All Staff capabilities plus: create/manage exam questions, import questions via CSV, generate exam versions |
| Administrator | Full system access: users, roles, products, store hours, tax config, dispatch assignment, health reports |

## Core Domains

### Product Catalogue
- SPU (Standard Product Unit) → SKU hierarchy
- Option groups with required/optional flags; option values carry price deltas
- Bilingual content (English + Chinese) on all product fields
- Categories and image URLs

### Cart & Checkout
- Per-user persistent cart with SKU-level line items
- Required option group enforcement at add-to-cart time
- Reservation locks prevent oversell during concurrent checkouts (10-minute hold)
- Pickup slot selection with prep-time enforcement derived from cart items
- Voucher codes generated at checkout, stored encrypted (AES-256-GCM); hash stored on reservation for scan verification
- Transactional order creation: reservation + order + order items + cart clear in a single DB transaction

### Order Lifecycle
```
Pending → Accepted → InPrep → Ready → Collected
                                    → Canceled (role-gated by state)
```
- Fulfilment state machine enforced in `services::fulfillment`
- Fulfilment events logged for full audit history
- Staff can scan pickup vouchers; mismatches are flagged and recorded

### Training & Exam Engine
- Subjects → Chapters → Questions hierarchy
- Question types: SingleChoice, MultipleChoice
- CSV bulk import for questions (Teacher role)
- Exam version generation: random selection from question pool by subject/chapter/difficulty
- Timed exam attempts with per-question answer recording
- Wrong-answer notebook: tracks incorrect answers across attempts for targeted review
- Review sessions surface questions due for re-practice
- Favourites: users can bookmark questions
- Score analytics: overall score, breakdown by subject and difficulty, recent attempt history

### Dispatch
- Station zones group staff and tasks geographically
- Two assignment modes: Grab (staff self-select from queue) and Assigned (admin pushes to specific staff)
- Task offer/accept/reject flow with configurable offer expiry (background job)
- Staff reputation scoring updated on task completion
- Shift windows track staff availability per zone per day
- Admin can view staff recommendations ranked by reputation score

### i18n
- Translation maps for `en` and `zh` served via `/api/i18n/translations/:locale`
- Users store a `preferred_locale` updated via `PUT /api/auth/locale`
- All product and exam content carries bilingual fields

### Security
- HMAC-signed rotating session cookies (`brewflow_session`): 30-min idle timeout, 5-min rotation window
- bcrypt password hashing with configurable cost
- AES-256-GCM voucher code encryption; only the hash is stored on the reservation row
- Sensitive fields (voucher codes, session IDs) masked in logs via `LogMaskFairing` middleware
- CORS restricted to origins configured via `ALLOWED_ORIGINS` env var (comma-separated); defaults to `http://localhost:8080`
- Password policy enforced at registration via `services::auth::validate_password`

### Resilience & Background Jobs
- `DegradationManager` tracks health of critical subsystems (sessions, reservations, analytics)
- `BackgroundJobManager` schedules and runs recurring jobs:
  - `session_cleanup` (every 5 min): removes expired sessions
  - `reservation_expiry` (every 60 s): expires stale pickup reservations
  - `offer_expiry` (every 15 s): expires stale dispatch task offers
  - `lock_cleanup` (every 60 s): releases expired reservation locks
  - `analytics_snapshot` (every 60 min): placeholder for analytics aggregation
- Readiness probe (`/health/ready`) returns 503 if any critical service is degraded

## Database

Migrations are applied in numeric order from `database/migrations/`:

| File | Domain |
|------|--------|
| 001_users_and_roles.sql | Users, roles, permissions |
| 002_products.sql | SPU/SKU catalogue, option groups, option values |
| 003_store_and_reservations.sql | Stores, pickup slots, reservations, vouchers |
| 004_cart_and_orders.sql | Cart, orders, order items, fulfilment events |
| 005_exam_system.sql | Subjects, chapters, questions, options, exam versions |
| 006_seed_data.sql | Initial seed data |
| 007_sessions_and_encryption.sql | Session storage, crypto config |
| 008_reservation_locks.sql | Inventory reservation locks |
| 009_dispatch_system.sql | Zones, task assignments, shifts, reputation |

## Service Ports

| Service | Port |
|---------|------|
| Rocket backend | 8000 |
| Dioxus frontend (dev) | 8080 |
