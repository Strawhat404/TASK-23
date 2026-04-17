# Browser E2E test runner — fully self-contained, no host dependencies.
FROM mcr.microsoft.com/playwright:v1.52.0-noble

WORKDIR /e2e
COPY package.json playwright.config.ts ./
RUN npm ci
COPY tests/ tests/

CMD ["npx", "playwright", "test"]
