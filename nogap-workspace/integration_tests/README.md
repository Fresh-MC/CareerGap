# Integration Tests

This directory contains integration tests for the NoGap project.

## CLI to Core Test (`cli_to_core_test.rs`)
Tests the integration between `nogap_cli` and `nogap_core`.

**Run with:**
```bash
cd /Users/sachin/Downloads/Project/NoGap/nogap-workspace
cargo test --test cli_to_core_test
```

## Dashboard Integration Test (`dashboard_integration.spec.ts`)
Tests the Tauri frontend-backend integration.

**Prerequisites:**
1. Install dependencies: `cd nogap_dashboard && npm install`
2. Start dev server: `npm run dev` (requires vite)
3. Run tests: `npx playwright test integration_tests/dashboard_integration.spec.ts`

**Note:** These are placeholder tests. Full implementation requires:
- Frontend Tauri bindings calling `invoke('get_version')` and `invoke('run_audit')`
- Dev server running (vite or similar)
- Proper async handling in Playwright tests
