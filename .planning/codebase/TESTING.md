# Testing Patterns

**Analysis Date:** 2026-02-15

## Test Framework

**Runner:**
- Vitest ^4.0.18 (dev dependency) - `package.json`
- Config: `vite.config.js` (test block)

**Assertion Library:**
- `@testing-library/jest-dom` (matchers) - `package.json`
- `@testing-library/svelte` (render utilities) - `package.json`

**Run Commands:**
```bash
# Not defined in `package.json` scripts
```

## Test File Organization

**Location:**
- Co-located with components: `src/lib/components/MainArea.test.ts` sits next to `src/lib/components/MainArea.svelte`.

**Naming:**
- `*.test.ts` pattern: `src/lib/components/MainArea.test.ts`.

**Structure:**
```
src/lib/components/
  MainArea.svelte
  MainArea.test.ts
```

## Test Structure

**Suite Organization:**
```typescript
import { render, screen } from "@testing-library/svelte";
import { describe, it, expect, beforeEach } from "vitest";
import { sessions, selectedSessionId } from "$lib/stores/sessions";
import MainArea from "./MainArea.svelte";

describe("MainArea", () => {
  beforeEach(() => {
    sessions.set([]);
    selectedSessionId.set(null);
  });

  it("shows empty state when there are no sessions", () => {
    render(MainArea);
    expect(screen.getByText("No active sessions")).toBeTruthy();
  });
});
```

**Patterns:**
- Use `beforeEach` to reset writable stores: `src/lib/components/MainArea.test.ts`.
- Use Testing Library `render` + `screen` queries: `src/lib/components/MainArea.test.ts`.

## Mocking

**Framework:** Not detected (no `vi.mock` usage in `src/lib/components/MainArea.test.ts`).

**Patterns:**
```typescript
// Not detected in current tests
```

**What to Mock:** Not established in codebase.

**What NOT to Mock:** Not established in codebase.

## Fixtures and Factories

**Test Data:**
```typescript
sessions.set([
  {
    id: "test-1",
    name: "Test Session",
    status: "running",
    working_dir: "/tmp",
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
  },
]);
```

**Location:**
- Inline objects inside test files: `src/lib/components/MainArea.test.ts`.

## Coverage

**Requirements:** None configured (no coverage settings in `vite.config.js`).

**View Coverage:**
```bash
# Not configured in `vite.config.js`
```

## Test Types

**Unit Tests:**
- Component rendering and store state assertions: `src/lib/components/MainArea.test.ts`.

**Integration Tests:** Not detected.

**E2E Tests:** Not used.

## Common Patterns

**Async Testing:** Not detected in current tests.

**Error Testing:** Not detected in current tests.

---

*Testing analysis: 2026-02-15*
