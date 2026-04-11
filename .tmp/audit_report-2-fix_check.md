# Five-Issue Verification 

Date: 2026-04-08  
Method: Static-only review

## Results

1. docs/api-spec.md mismatches implemented store/training routes  
- Status: **Fixed**  
- Evidence:
  - Training docs use `/api/training/modules` and `/api/exams/...`: `docs/api-spec.md:80-84`
  - Store docs us `/api/store` and `/api/training`: `repo/backend/src/main.rs:167-169`

2. Product page tax hardcoded to 0.08 instead of backend config  
- Status: **Fixed**  
- Evidence:
  - Product page fetches tax from `/store/tax`: `repo/frontend/src/pages/product.rs:40-43`
  - Uses backend `cfg.rate` as tax rate: `repo/frontend/src/pages/product.rs:53-58`
  - Computes tax from dynamic `tax_rate`: `repo/frontend/src/pages/product.rs:77`

3. Dispatch recommendation missing shift time-window filtering  
- Status: **Fixed**  
- Evidence:
  - Explicit filter keeps only shifts covering current time: `repo/backend/src/services/dispatch.rs:72-85`

4. README JWT-fallback text conflicts with cookie-only guard  
- Status: **Fixed**  
- Evidence:
  - README now documents cookie replay, not JWT fallback: `repo/README.md:92-95`
  - Guard remains cookie-only: `repo/backend/src/middleware/auth_guard.rs:14-17`

5. DB integration tests still self-skip when DB env missing  
- Status: **Fixed (self-skip behavior removed from tests)**  
- Evidence:
  - DB test macro now panics when env missing instead of returning early: `repo/backend/src/api_tests.rs:124-135`
  - No-DB behavior is handled by test runner with explicit `--skip` filters: `repo/run_tests.sh:34-42`

## Final
- Fixed: **5/5**

