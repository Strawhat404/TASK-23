# BriefFlow — System Design

## Overview

BriefFlow is an offline-first retail ordering and internal training platform for a multi-location food and beverage business. It runs entirely on the local network with no external service dependencies.

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
| Customer | Browse catalogue, manage cart, place orders, apply vouchers, track order status |
| Staff | View and transition order fulfilment states, manage dispatch tasks |
| Teacher | Create/manage training modules and exams |
| Administrator | Full system access: users, products, stores, promotions, configuration |

## Core Domains

### Product Catalogue
- SPU (Standard Product Unit) → SKU hierarchy
- Categories, tags, images
- Pricing with promotion/discount support

### Cart & Checkout
- Reservation locks prevent oversell
- Pickup slot selection with prep-time enforcement
- Voucher application (hash-at-rest with AES-256-GCM)
- Transactional order creation

### Order Lifecycle
```
Pending → Accepted → Preparing → Ready → Collected
                                       → Cancelled (role-gated)
```

### Training & Exam Engine
- Modules with lessons and media attachments
- Timed exams with multiple question types
- Attempt tracking and score history

### Dispatch
- Task assignment to staff members
- Status transitions with ownership checks

### Security
- HMAC-signed rotating session cookies (30-min idle, 5-min rotation)
- bcrypt password hashing
- AES-256-GCM voucher encryption
- Request log masking for sensitive fields

## Database

Migrations are applied in numeric order from `database/migrations/`:

| File | Domain |
|------|--------|
| 001_users_and_roles.sql | Users, roles, permissions |
| 002_products.sql | SPU/SKU catalogue |
| 003_store_and_reservations.sql | Stores, pickup slots, reservations |
| 004_cart_and_orders.sql | Cart, orders, line items |
| 005_exam_system.sql | Training modules, exams, attempts |
| 006_seed_data.sql | Initial seed data |
| 007_sessions_and_encryption.sql | Session storage |
| 008_reservation_locks.sql | Inventory reservation locks |
| 009_dispatch_system.sql | Dispatch tasks |

## Service Ports

| Service | Port |
|---------|------|
| Rocket backend | 8000 |
| Dioxus frontend (dev) | 8080 |
