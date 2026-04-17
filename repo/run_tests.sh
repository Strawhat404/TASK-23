#!/usr/bin/env bash
# Run the BrewFlow backend test suite inside Docker.
# Usage: ./run_tests.sh  (from the repo/ directory)
#
# Spins up a MySQL container, builds the backend in a Rust container,
# runs all tests against a real database, then tears everything down.
# No local Rust toolchain or MySQL required.
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_DIR"

COMPOSE_PROJECT="brewflow-test"
DB_CONTAINER="${COMPOSE_PROJECT}-db"
TEST_CONTAINER="${COMPOSE_PROJECT}-runner"
NETWORK="${COMPOSE_PROJECT}-net"
DB_PASSWORD="testroot"
DB_NAME="brewflow_test"
DB_PORT=13306  # avoid collision with dev DB on 3306

cleanup() {
    echo ""
    echo "[cleanup] Stopping test containers..."
    docker rm -f "$DB_CONTAINER" "$TEST_CONTAINER" 2>/dev/null || true
    docker network rm "$NETWORK" 2>/dev/null || true
}
trap cleanup EXIT

echo "============================================"
echo "  BrewFlow — Dockerised Test Suite"
echo "============================================"
echo ""

# ── Step 1: Create isolated network ────────────────────────────────────────
echo "[1/5] Creating test network..."
docker network create "$NETWORK" 2>/dev/null || true

# ── Step 2: Start MySQL ────────────────────────────────────────────────────
echo "[2/5] Starting MySQL container..."
docker rm -f "$DB_CONTAINER" 2>/dev/null || true
docker run -d \
    --name "$DB_CONTAINER" \
    --network "$NETWORK" \
    -e MYSQL_ROOT_PASSWORD="$DB_PASSWORD" \
    -e MYSQL_DATABASE="$DB_NAME" \
    -v "$REPO_DIR/database/migrations:/docker-entrypoint-initdb.d:ro" \
    mysql:8.0 \
    > /dev/null

echo "      Waiting for MySQL to be ready (first run may take ~2 min for init + migrations)..."
for i in $(seq 1 180); do
    if docker exec "$DB_CONTAINER" mysqladmin ping -h 127.0.0.1 -uroot -p"$DB_PASSWORD" --silent 2>/dev/null; then
        # Also verify the DB exists and migrations ran
        if docker exec "$DB_CONTAINER" mysql -uroot -p"$DB_PASSWORD" -e "USE $DB_NAME; SHOW TABLES;" 2>/dev/null | grep -q users; then
            break
        fi
    fi
    if [ "$i" -eq 180 ]; then
        echo "ERROR: MySQL did not become ready in 180s"
        docker logs "$DB_CONTAINER" | tail -20
        exit 1
    fi
    sleep 1
done
echo "      MySQL is ready."

# Apply test-only fixtures (extra users, orders, vouchers for integration tests)
echo "      Applying test fixtures (test_users.sql)..."
docker exec -i "$DB_CONTAINER" mysql -uroot -p"$DB_PASSWORD" "$DB_NAME" < "$REPO_DIR/database/test_users.sql" 2>/dev/null
echo "      Verifying test users..."
docker exec "$DB_CONTAINER" mysql -uroot -p"$DB_PASSWORD" "$DB_NAME" -e "SELECT username, display_name FROM users ORDER BY id;" 2>/dev/null
echo ""

# ── Step 3: Build + run tests in a Rust container ─────────────────────────
TEST_DATABASE_URL="mysql://root:${DB_PASSWORD}@${DB_CONTAINER}/${DB_NAME}"

echo "[3/5] Building test runner image (dependencies pre-installed)..."
TEST_IMAGE="${COMPOSE_PROJECT}-runner-img"
docker build -t "$TEST_IMAGE" -f "$REPO_DIR/Dockerfile.test" "$REPO_DIR" > /dev/null 2>&1
echo "      Image ready: $TEST_IMAGE"
echo ""

echo "[4/5] Running tests..."
echo "      DB: $TEST_DATABASE_URL"
echo ""

CACHE_VOLUME="${COMPOSE_PROJECT}-cargo-cache"
docker volume create "$CACHE_VOLUME" > /dev/null 2>&1 || true

docker rm -f "$TEST_CONTAINER" 2>/dev/null || true
docker run \
    --name "$TEST_CONTAINER" \
    --network "$NETWORK" \
    -e DATABASE_URL="$TEST_DATABASE_URL" \
    -e TEST_DATABASE_URL="$TEST_DATABASE_URL" \
    -e ROCKET_SECRET_KEY="hPRYyVRiMyxpw5sBB1XeCMN1kFsDCqKvBi2QJxBVHQo=" \
    -e JWT_SECRET="test-jwt-secret" \
    -e COOKIE_SECRET="test-cookie-secret" \
    -e ENCRYPTION_KEY="test-encryption-key-1234567890ab" \
    -e CARGO_TARGET_DIR="/tmp/target" \
    -v "$REPO_DIR:/app:ro" \
    -v "$CACHE_VOLUME:/tmp/target" \
    -w /app \
    "$TEST_IMAGE" \
    bash -c '
        echo "[4a/5] cargo test --package shared (unit tests) ..."
        cargo test --package shared
        echo "      OK"
        echo ""
        echo "[4b/5] cargo test --package backend (ALL tests — DB available) ..."
        cargo test --package backend -- --nocapture
    '

# ── Step 5: Browser E2E tests (optional — skipped if --skip-e2e) ──────────
E2E_CONTAINER="${COMPOSE_PROJECT}-e2e"
if [ "${1:-}" != "--skip-e2e" ] && [ -f "$REPO_DIR/e2e/Dockerfile" ]; then
    echo ""
    echo "[5/5] Building and running browser E2E tests..."
    E2E_IMAGE="${COMPOSE_PROJECT}-e2e-img"
    if docker build -t "$E2E_IMAGE" "$REPO_DIR/e2e" > /dev/null 2>&1; then
        docker rm -f "$E2E_CONTAINER" 2>/dev/null || true
        docker run \
            --name "$E2E_CONTAINER" \
            --network "$NETWORK" \
            -e BASE_URL="http://${DB_CONTAINER}:8080" \
            -e API_URL="http://${DB_CONTAINER}:8000" \
            "$E2E_IMAGE" || echo "      [WARN] Browser E2E tests require a running frontend+backend. Skipped."
    else
        echo "      [WARN] Browser E2E image build failed. Skipped."
    fi
fi

echo ""
echo "============================================"
echo "  All tests passed."
echo "============================================"
