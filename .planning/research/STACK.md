# Stack Research

**Domain:** Native Desktop AI Agent Orchestrator
**Researched:** 2026-02-14
**Confidence:** HIGH

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Tauri v2 | 2.10.x | Desktop application framework (Rust backend) | Industry standard for secure, performant desktop apps. Stable v2 released Oct 2024, actively maintained with regular patches. Native system integration, small binary size (~600KB), and security-first architecture with IPC isolation. |
| Svelte 5 | 5.x (stable) | Frontend framework | Released Oct 2024 with revolutionary runes reactivity system. 15-30% smaller bundles than Svelte 4, native TypeScript support without preprocessors. Perfect for real-time UIs with streaming output. |
| SvelteKit | Latest (compatible with Svelte 5) | Full-stack web framework | Official Svelte meta-framework. SPA mode works seamlessly with Tauri. Provides routing, state management via stores, and server-side utilities (though server features unused in Tauri context). |
| Rust | 1.77.2+ | Backend language | Required by Tauri. Memory-safe, fast, excellent async support via tokio. Strong ecosystem for systems programming. |
| Bun | 1.x | Package manager & runtime | Fast (2-3x npm speed), native TypeScript support, compatible with Tauri v2 as of Tauri 1.5. Note: Some compatibility edge cases with latest Bun versions - verify bun.lock support in current Tauri CLI. |
| Vite | 7.x | Build tool | Official recommendation from Tauri. Uses Rollup for production bundles. 5x faster full builds, 100x faster incremental builds than alternatives. Node.js 20.19+ required for Vite 7. |
| TypeScript | 5.x | Type system | Native support in Svelte 5 and Tauri. Type-safe IPC between frontend/backend. Improved DX and catches errors at compile time. |

### Database

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| SQLite | 3.x | Session persistence | Embedded database, no separate server needed. Perfect for desktop apps. Use via tauri-plugin-sql (official) or tauri-plugin-rusqlite2 (community, more direct rusqlite access). |
| tauri-plugin-sql | 2.3.x | SQLite integration | Official Tauri plugin. Uses sqlx under the hood, supports migrations, async/await. Rust 1.77.2+ required. |
| sqlx | 0.7.x (via tauri-plugin-sql) | SQL toolkit | Type-checked queries at compile time, async, connection pooling. Industry standard for Rust SQL access. |

### Rust Backend Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| cc-sdk | 0.1.10+ | Claude Code CLI integration | Core integration for orchestrating Claude Code sessions. Provides async clients, streaming, interrupt, control protocol, token optimization. Built on tokio. |
| tokio | 1.x | Async runtime | Used by Tauri by default. Tauri owns the runtime - don't add #[tokio::main] unless you need custom control. Spawns async commands automatically. |
| serde | 1.x | Serialization | Core to Tauri IPC. All command arguments/returns must impl Serialize/Deserialize. Standard for Rust serialization. |
| serde_json | 1.x | JSON serialization | IPC communication format between frontend and Rust backend. |
| thiserror | 2.x | Error type definitions | Define custom error types for your internal APIs. Minimal boilerplate, derives Error trait. Use in library code. |
| anyhow | 2.x | Error handling & context | Application-level error handling. Adds context to errors, simplifies propagation. Use in main.rs and top-level code. |

### Frontend Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| @tauri-apps/api | 2.x | Tauri frontend API | IPC invoke() calls, events, window management, file system access. Required for all Tauri frontend communication. |
| shadcn-svelte | Latest (Svelte 5 compatible) | UI component library | Copy-paste components (not npm dependency). Native Svelte 5 support. Built with Tailwind CSS. Gives you full control to customize. 7,500+ GitHub stars. |
| Tailwind CSS | 4.x | Styling framework | Major v4 stable release (Jan 2025). CSS-based config, 5x faster builds, 100x faster incremental. Utility-first CSS. Integrates perfectly with Svelte. |
| Zod | 3.x | Schema validation | Type-safe validation for forms, IPC payloads, config files. 60% faster than alternatives (2.5ms vs 6.2ms). Pairs well with TypeScript. Use with Superforms for form handling. |
| Superforms | 2.x | Form library | SvelteKit form library with Zod integration. Client and server validation, type-safe, optimistic UI support. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| @tauri-apps/cli | Tauri CLI tooling | Install as dev dependency: `bun add -D @tauri-apps/cli@^2.10.0`. Handles dev server, builds, code signing. |
| Vitest | Unit testing (frontend) | Modern test runner. Mock Tauri IPC with mockIPC. Use for testing Svelte components and business logic. |
| WebDriver / Playwright | E2E testing | WebDriver for full system testing. Playwright for UI automation. Mock IPC for frontend-only tests. |
| rust-analyzer | Rust LSP | Essential for Rust development. Type hints, completions, inline errors. |
| Svelte for VS Code | Svelte LSP | Svelte syntax, type checking in .svelte files, runes support. |
| ESLint | JavaScript/TypeScript linting | Configure for Svelte. Catch common mistakes. |
| Prettier | Code formatting | Svelte plugin for consistent formatting. |

## Installation

### Prerequisites

```bash
# macOS prerequisites (Homebrew)
brew install rust
brew install node # Or install via nvm

# Verify versions
rustc --version  # Should be 1.77.2 or higher
node --version   # Should be 20.19 or higher for Vite 7

# Install Bun
curl -fsSL https://bun.sh/install | bash
```

### Project Setup

```bash
# Create Tauri app with Svelte template
bun create tauri-app lulu --template svelte-ts

cd lulu

# Install Tauri CLI as dev dependency
bun add -D @tauri-apps/cli@^2.10.0

# Install Tauri API for frontend
bun add @tauri-apps/api

# Install SQLite plugin
cargo add tauri-plugin-sql --features sqlite

# Install Claude Code SDK
cargo add cc-sdk@0.1.10
cargo add tokio --features full
cargo add serde --features derive
cargo add serde_json
cargo add thiserror@2
cargo add anyhow@2

# Frontend dependencies
bun add zod
bun add -D tailwindcss@4 autoprefixer postcss
bun add -D vitest @testing-library/svelte

# Initialize Tailwind CSS v4
bunx tailwindcss init -p
```

### Tailwind CSS v4 Setup

In your main CSS file (e.g., `src/app.css`):

```css
@import "tailwindcss";
```

That's it for Tailwind v4 - no tailwind.config.js needed by default. Customize via CSS variables if needed.

### shadcn-svelte Setup

```bash
bunx shadcn-svelte@latest init
```

Follow prompts to configure. Components are copied into your project (not installed as dependencies).

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Tauri v2 | Electron | If you need maximum ecosystem compatibility, Chrome DevTools, or are porting an existing Electron app. Trade-off: 100MB+ bundle size, higher memory usage. |
| Svelte 5 | React 19 | If team has deep React expertise or needs React-specific libraries. Trade-off: Larger bundles, more boilerplate, no native reactivity. |
| SvelteKit | Vite + Svelte | If you want minimal abstractions and only need SPA features. Trade-off: Lose routing utilities, form actions, load functions. |
| Bun | pnpm/npm | If Bun compatibility issues arise or CI/CD doesn't support Bun. pnpm is fast and stable. npm is universal. |
| tauri-plugin-sql | tauri-plugin-rusqlite2 | If you need direct rusqlite API access or custom SQLite extensions. Trade-off: Less official support, manual migration management. |
| sqlx | rusqlite | If you prefer synchronous API or need advanced SQLite features. Trade-off: No compile-time query checking, less async-friendly. |
| Tailwind CSS | CSS Modules / Sass | If you prefer traditional CSS or have existing stylesheets. Trade-off: More custom CSS to write, less consistent design system. |
| shadcn-svelte | Skeleton / Flowbite-Svelte | Skeleton for Figma integration and design system. Flowbite for pre-styled Material-style components. Trade-off: Less customization control (installed dependencies). |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| Tauri v1 | Outdated. No mobile support, older IPC system, missing v2 features (raw payloads, channels). End of life expected 2026. | Tauri v2 |
| Svelte 4 | Old reactivity model (stores only). Missing runes, slower performance, larger bundles. | Svelte 5 |
| Vite 4 or older | Slower build times, outdated Rollup version, missing performance optimizations. | Vite 7.x |
| Tailwind CSS v3 | Requires JavaScript config file, slower builds, missing cascade layers and v4 features. | Tailwind CSS v4 |
| #[tokio::main] in Tauri | Tauri owns the async runtime by default. Adding this causes conflicts unless you explicitly need custom runtime control. | Let Tauri manage tokio runtime |
| Webpack | Slower than Vite, more complex config, not recommended by Tauri docs. | Vite |
| Jest | Slower than Vitest, more config needed, worse ESM support. | Vitest |
| yarn / npm (unless needed) | Slower than Bun/pnpm, larger node_modules. | Bun or pnpm |
| Global state in stores (SSR context) | SvelteKit stores are client-only. Using on server shares state across requests (security risk). | Use $state runes with context API for SSR-safe state |

## Stack Patterns by Variant

**If building multi-session dashboard with real-time streaming:**
- Use Svelte 5 runes ($state, $derived, $effect) for fine-grained reactivity
- Leverage Tauri IPC Channels for streaming output from cc-sdk
- Store session state in SQLite, keep active sessions in memory with $state
- Use Web Workers for heavy processing if needed (terminal parsing, etc.)

**If implementing auto-approve rules:**
- Store rules in SQLite with tauri-plugin-sql
- Validate rule schemas with Zod on both frontend and backend
- Use Rust pattern matching for rule evaluation (fast, type-safe)
- Emit events from Rust to frontend when auto-approve triggers

**If adding interrupt/resume functionality:**
- cc-sdk supports interrupts via control protocol
- Store interrupt state in SQLite for persistence
- Use Tauri state management (Arc<Mutex<T>>) for in-memory session state
- Emit progress events to frontend via Tauri event system

**If implementing session persistence:**
- SQLite schema: sessions table (id, name, created_at, state, config)
- Use sqlx migrations for schema versioning
- Serialize session config as JSON in SQLite (serde_json)
- Load on app startup, save on session changes (debounced)

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| Tauri 2.10.x | Rust 1.77.2+ | Minimum Rust version enforced by plugins |
| Tauri 2.10.x | Node 20.19+ | For Vite 7 compatibility |
| Svelte 5.x | SvelteKit latest | Use SvelteKit versions released after Oct 2024 |
| Vite 7.x | Node 20.19+, 22.12+ | Dropped Node 18 support |
| Tailwind CSS v4 | PostCSS 8+ | Requires PostCSS for build process |
| shadcn-svelte | Svelte 5 | Ensure you use Svelte 5 compatible version |
| cc-sdk 0.1.10 | tokio 1.x | Built on tokio, async/await required |
| tauri-plugin-sql 2.3.x | Rust 1.77.2+ | Documented minimum version |
| Bun 1.x | Tauri 1.5+ | Bun support added in Tauri 1.5, verify bun.lock compatibility in latest Tauri CLI |

## Security Considerations

**Tauri IPC:**
- Use Tauri's capability system (allowlist) to restrict which commands frontend can call
- Validate all inputs in Rust commands (never trust frontend data)
- Use serde for type-safe serialization, reducing injection risks

**SQLite:**
- Use parameterized queries via sqlx to prevent SQL injection
- Store sensitive data encrypted (use Tauri's keyring plugin for credentials)
- Set proper file permissions on SQLite database

**Code Signing (macOS):**
- Requires Apple Developer ID Application certificate ($99/year)
- Free developer accounts can't notarize (shows "unverified" warning)
- Configure in tauri.conf.json under bundle.macOS.signingIdentity
- Use environment variables for CI/CD: APPLE_CERTIFICATE, APPLE_ID, APPLE_PASSWORD

## Performance Optimizations

**Frontend:**
- Lazy load routes/components with dynamic imports
- Use Svelte's built-in reactivity (avoid manual DOM manipulation)
- Debounce expensive operations (search, auto-save)
- Virtual scrolling for large lists (session history, logs)

**Backend:**
- Use async/await for all I/O (file system, SQLite, cc-sdk calls)
- Connection pooling for SQLite (via sqlx)
- Batch database operations where possible
- Stream large data via IPC Channels (not JSON serialization)

**IPC:**
- Minimize IPC calls (batch operations)
- Use raw payloads for large data (images, logs) instead of JSON
- Cache frequently accessed data on frontend

## Sources

### High Confidence (Official Docs & Context7)

- [Tauri v2 Official Docs](https://v2.tauri.app/) — Architecture, IPC, plugins
- [Tauri 2.0 Stable Release](https://v2.tauri.app/blog/tauri-20/) — Release notes, new features
- [Tauri Releases](https://github.com/tauri-apps/tauri/releases) — Version history
- [Svelte 5 Release](https://svelte.dev/blog/svelte-5-is-alive) — Runes, reactivity system
- [Svelte 5 Docs](https://svelte.dev/docs/svelte/v5-migration-guide) — Migration guide, features
- [Tauri SQL Plugin](https://v2.tauri.app/plugin/sql/) — Official SQLite integration
- [Tauri IPC Documentation](https://v2.tauri.app/concept/inter-process-communication/) — IPC system, raw payloads
- [Tauri API Reference](https://v2.tauri.app/reference/javascript/api/) — TypeScript API docs
- [Vite Documentation](https://vite.dev/) — Build tool, configuration
- [Vite 7 Release](https://vite.dev/blog/announcing-vite7) — New features, requirements
- [Tailwind CSS v4](https://tailwindcss.com/blog/tailwindcss-v4) — Release notes, migration
- [shadcn-svelte](https://shadcn-svelte.com/) — Component library documentation
- [cc-sdk on crates.io](https://crates.io/crates/cc-sdk) — Package information
- [@tauri-apps/cli npm](https://www.npmjs.com/package/@tauri-apps/cli) — Version history

### Medium Confidence (Community Resources)

- [Tauri + Svelte 5 templates on GitHub](https://github.com/alysonhower/tauri2-svelte5-shadcn) — Community examples
- [Tauri + Svelte Best Practices (Medium)](https://medium.com/@puneetpm/native-apps-reimagined-why-tauri-rust-and-svelte-is-my-go-to-stack-in-2025-209f5b2937a1) — Stack recommendations
- [SQLite in Tauri Tutorial](https://dezoito.github.io/2025/01/01/embedding-sqlite-in-a-tauri-application.html) — Implementation patterns
- [SvelteKit State Management Guide](https://svelte.dev/docs/kit/state-management) — Official guide
- [Svelte 5 Runes Global State](https://mainmatter.com/blog/2025/03/11/global-state-in-svelte-5/) — Best practices
- [Rust Error Handling 2025](https://markaicode.com/rust-error-handling-2025-guide/) — thiserror/anyhow patterns
- [Superforms Documentation](https://superforms.rocks/) — Form validation with Zod
- [Vitest for Tauri](https://yonatankra.com/how-to-setup-vitest-in-a-tauri-project/) — Testing setup
- [Tauri macOS Code Signing](https://v2.tauri.app/distribute/sign/macos/) — Official signing guide

---

*Stack research for: Lulu - Claude Code Orchestrator*
*Researched: 2026-02-14*
*Next: Feed into roadmap creation for phase structure and dependency ordering*
