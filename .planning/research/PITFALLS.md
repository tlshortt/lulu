# Pitfalls Research

**Domain:** Tauri v2 + cc-sdk Multi-Session AI Agent Orchestrator
**Researched:** 2026-02-14
**Confidence:** MEDIUM

## Critical Pitfalls

### Pitfall 1: SQLite Write Serialization Bottleneck

**What goes wrong:**
Multiple concurrent cc-sdk sessions attempt to persist state to SQLite simultaneously, causing SQLITE_BUSY errors and write operation failures. Sessions block each other, leading to degraded performance and potential data loss.

**Why it happens:**
SQLite fundamentally supports only one writer at any instant, even in WAL mode. Developers assume WAL mode provides concurrent writes, but it only provides concurrent *reads* with a single writer. DEFERRED transactions cause upgrade deadlocks that return SQLITE_BUSY immediately without respecting timeout settings.

**How to avoid:**
- Use BEGIN IMMEDIATE for all write transactions (prevents upgrade deadlocks)
- Implement a write queue with a single writer thread per database connection
- Set busy_timeout to appropriate milliseconds (e.g., 5000ms minimum)
- Consider connection pooling with write serialization
- For truly concurrent writes, evaluate Turso (Rust SQLite rewrite with MVCC)

**Warning signs:**
- SQLITE_BUSY errors in logs
- Transactions timing out unpredictably
- Session persistence operations blocking for extended periods
- Database lock contention during multi-session stress tests

**Phase to address:**
Phase 1 (Core Architecture) - Database layer must be designed correctly from the start. Retrofitting write serialization is expensive.

---

### Pitfall 2: Unbounded Channel Memory Exhaustion

**What goes wrong:**
Streaming output from multiple cc-sdk sessions flows through unbounded Tokio channels. Fast-producing sessions (verbose AI output) overwhelm slow consumers (UI rendering), causing unbounded memory growth until OOM crash.

**Why it happens:**
Unbounded channels lack backpressure mechanisms. If producers (cc-sdk stdout/stderr streams) generate data faster than consumers (Tauri IPC, UI updates) can process it, the channel grows indefinitely. This is particularly dangerous with streaming AI output, which can be extremely verbose.

**How to avoid:**
- NEVER use unbounded channels in production
- Use bounded channels with capacity limits (e.g., 1000 messages per session)
- Implement backpressure by making producers wait when channel is full
- Monitor channel capacity metrics and alert on >80% utilization
- Consider rate limiting output streams before enqueueing

**Warning signs:**
- Memory usage growing linearly with session runtime
- Gradual performance degradation over extended sessions
- UI freezing when switching between sessions (backlog processing)
- Channel buffer sizes growing without bounds in metrics

**Phase to address:**
Phase 1 (Core Architecture) - Stream handling architecture must include backpressure from inception. Changing channel types later requires significant refactoring.

---

### Pitfall 3: IPC Serialization Blocking Async Runtime

**What goes wrong:**
Large JSON payloads (session state, long AI responses) serialize synchronously on Tokio worker threads, blocking the async runtime. Other sessions stall waiting for CPU time, causing cascading latency and perceived hangs.

**Why it happens:**
Tauri v2 requires all IPC data to be JSON-serializable. Large structures (conversation history, complex state) serialize synchronously during send operations. One blocking call grinds the entire async runtime to a halt because Tokio workers can't swap tasks during CPU-bound work.

**How to avoid:**
- Offload large serialization to Tokio's blocking threadpool (`spawn_blocking`)
- Chunk large payloads (e.g., send conversation history in pages, not all at once)
- Limit IPC payload sizes (set max 1MB per message)
- Use incremental streaming for large responses instead of batching
- Monitor task poll times and alert on >10ms polls

**Warning signs:**
- All sessions becoming unresponsive when one session sends large data
- CPU spikes correlating with IPC sends
- Task poll times exceeding 10ms in Tokio diagnostics
- UI freezing during session state synchronization

**Phase to address:**
Phase 1 (Core Architecture) - IPC design patterns must be established early. Streaming vs. batching decisions affect the entire system.

---

### Pitfall 4: Approval Workflow Deadlock

**What goes wrong:**
An approval request is sent to the frontend, but the async task awaits approval while holding a lock (e.g., session state mutex). If the user never approves, or if the approval response requires the same lock, the system deadlocks.

**Why it happens:**
Approval workflows naturally block execution, but async tasks holding shared resources (locks, channels) while awaiting user input create deadlock scenarios. The blocking nature of user interaction conflicts with async runtime expectations.

**How to avoid:**
- NEVER hold locks across approval await points
- Use message-passing for approval (send request, release all locks, await response)
- Implement approval timeouts (e.g., 5 minutes) with default deny
- Design approval state machine separately from session state
- Consider "optimistic execution" with rollback on denial

**Warning signs:**
- Sessions becoming permanently unresponsive after approval requests
- Deadlock detection tools reporting circular waits
- Multiple sessions blocking on the same resource
- Approval UI becoming unresponsive when clicked

**Phase to address:**
Phase 2 (Approval System) - Critical to design correctly. Deadlocks are hard to reproduce and debug after deployment.

---

### Pitfall 5: cc-sdk Process Zombie Accumulation

**What goes wrong:**
cc-sdk subprocess crashes or is forcefully terminated, but the parent Rust process never reaps the child. Zombie processes accumulate, consuming process table entries until the OS refuses to spawn new processes.

**Why it happens:**
The cc-sdk uses subprocess transport for communication. If processes crash unexpectedly (segfault, SIGKILL), and the parent doesn't properly wait() on them, they remain as zombies. This is exacerbated by interrupt/resume cycles that don't clean up properly.

**How to avoid:**
- Register signal handlers for SIGCHLD to reap children immediately
- Use `tokio::process::Command` with automatic reaping
- Implement process lifecycle tracking with timeout-based cleanup
- Wrap all subprocess operations in RAII guards that ensure cleanup
- Monitor zombie process count and alert

**Warning signs:**
- `ps aux | grep defunct` shows growing zombie count
- New session spawns failing with "resource temporarily unavailable"
- Process table exhaustion errors in logs
- OS refusing to fork new processes

**Phase to address:**
Phase 1 (Core Architecture) - Process lifecycle management is foundational. Fixing zombie leaks requires architectural changes.

---

### Pitfall 6: SIGSTOP/SIGCONT Process Interrupt Brittleness

**What goes wrong:**
Pausing a cc-sdk session with SIGSTOP leaves the process mid-operation. When resumed with SIGCONT, the subprocess is in an inconsistent state (partial write, interrupted syscall), causing crashes or data corruption.

**Why it happens:**
SIGSTOP cannot be caught or ignored - it forcefully pauses execution at arbitrary points. If the subprocess was mid-write to SQLite, mid-IPC send, or holding internal locks, resuming doesn't guarantee correct recovery. Additionally, SIGSTOP/SIGCONT are untrappable in Rust, so cleanup code never runs.

**How to avoid:**
- AVOID SIGSTOP/SIGCONT for pause/resume - use graceful cooperative suspension instead
- Design pause protocol: send "pause" message, wait for subprocess acknowledgment, then pause
- Implement resume handshake: resume process, wait for "ready" signal before sending new work
- Use timeout-based recovery if resume fails (kill and restart session)
- Persist session state before any pause operation

**Warning signs:**
- Sessions crashing immediately after resume
- SQLite corruption errors after pause/resume cycles
- Partial messages in IPC streams after resume
- Subprocess hanging after SIGCONT

**Phase to address:**
Phase 2 (Session Lifecycle) - Interrupt/resume is a core feature. Getting it wrong breaks user trust and causes data loss.

---

### Pitfall 7: Frontend State Synchronization Race Condition

**What goes wrong:**
Multiple sessions emit state updates concurrently. The Svelte frontend receives updates out of order, displaying stale data or creating UI inconsistencies (wrong session shows active, message counts incorrect).

**Why it happens:**
Tauri IPC events are asynchronous and unordered. If Session A emits "status: running" followed by "status: complete," but Session B emits updates simultaneously, the frontend may process B's update, then A's "running," showing A as active when it's actually complete.

**How to avoid:**
- Include monotonic version/sequence numbers with every state update
- Frontend discards updates with version < current_version for that session
- Use optimistic UI updates with eventual consistency reconciliation
- Batch state updates into atomic snapshots (not incremental patches)
- Implement Svelte stores with update deduplication logic

**Warning signs:**
- UI showing sessions as "running" when they're actually complete
- Message counts jumping backward
- Session list flickering between different states
- User reports of "UI is lying about session status"

**Phase to address:**
Phase 3 (Multi-Session UI) - Frontend state management must be designed for concurrent updates from inception.

---

### Pitfall 8: cc-sdk v0.5 API Surface Instability

**What goes wrong:**
Lulu integrates deeply with cc-sdk v0.5, a relatively new crate. Minor version updates introduce breaking changes, deprecations, or behavior changes that break Lulu's orchestration logic.

**Why it happens:**
cc-sdk is pre-1.0 (currently v0.5), meaning semantic versioning allows breaking changes in minor versions. The crate is actively developed with APIs still stabilizing. Deep integration with unstable APIs creates maintenance burden.

**How to avoid:**
- Pin exact cc-sdk version in Cargo.toml (no `^` or `~` version ranges)
- Abstract cc-sdk behind internal facade/adapter layer
- Monitor cc-sdk GitHub for breaking change announcements
- Contribute to cc-sdk to influence API stability
- Maintain integration tests that fail on API changes

**Warning signs:**
- `cargo update` breaks builds
- Behavior changes in new cc-sdk versions without code changes
- Deprecation warnings accumulating
- GitHub issues reporting cc-sdk API instability

**Phase to address:**
Phase 1 (Core Architecture) - Abstraction layer must exist before building on top. Retrofitting abstraction is expensive.

---

### Pitfall 9: Stateful ClaudeSDKClient Cross-Contamination

**What goes wrong:**
ClaudeSDKClient instances for different sessions accidentally share state (conversation history, token budgets, hooks), causing responses from Session A to appear in Session B's output.

**Why it happens:**
ClaudeSDKClient is stateful - each instance maintains conversation context. If instances are shared, pooled incorrectly, or if static/global state leaks between instances, sessions contaminate each other. Token budgets and hook callbacks may also be shared unintentionally.

**How to avoid:**
- One ClaudeSDKClient instance per session - never share or pool
- Wrap clients in session-scoped structs with explicit ownership
- Avoid static/global state in hook callbacks
- Implement session ID tagging for all logs/metrics to detect leaks
- Test with concurrent sessions sending distinct queries and verify output isolation

**Warning signs:**
- Messages from one session appearing in another's output
- Token usage counts shared across sessions
- Hook callbacks receiving events from wrong sessions
- Conversation history mixing between sessions

**Phase to address:**
Phase 1 (Core Architecture) - Session isolation is foundational. Cross-contamination destroys product integrity.

---

### Pitfall 10: Tauri Window/WebView Process Lifecycle Mismatch

**What goes wrong:**
User closes the Tauri window, expecting the app to quit, but background cc-sdk sessions continue running. Or conversely, sessions are killed when the window closes, losing in-progress work.

**Why it happens:**
Tauri's multi-process model separates Core (Rust) from WebView (frontend). Window close events don't automatically terminate Core processes. Without explicit lifecycle management, sessions persist as orphans or are brutally terminated.

**How to avoid:**
- Hook window close events to trigger graceful session shutdown
- Implement "minimize to tray" to avoid accidental closures
- Show confirmation dialog if active sessions exist on close
- Persist session state on window close for resume-on-reopen
- Separate session lifecycle from window lifecycle explicitly

**Warning signs:**
- Orphaned cc-sdk processes after app "closes"
- Sessions lost when user accidentally closes window
- Users complaining "app won't quit"
- Resource leaks accumulating over multiple window open/close cycles

**Phase to address:**
Phase 3 (Multi-Session UI) - Window lifecycle integration is a UX cornerstone. Getting it wrong frustrates users.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Using unbounded channels | Simpler code, no backpressure handling | Memory exhaustion, OOM crashes | Never in production |
| DEFERRED transactions | Slightly faster for read-heavy workloads | Upgrade deadlocks, SQLITE_BUSY errors | Never with concurrent writes |
| Shared ClaudeSDKClient pool | Reduces client initialization overhead | Session cross-contamination, race conditions | Never - always use per-session clients |
| No approval timeouts | Simpler approval logic | Deadlocks, sessions hanging indefinitely | Only in MVP with single-user testing |
| SIGSTOP for pause/resume | Quick to implement, OS-level feature | Process corruption, unrecoverable states | Never - always use cooperative suspension |
| Batched IPC payloads | Fewer IPC calls, reduced overhead | Blocking serialization, latency spikes | Acceptable for small payloads (<100KB) |
| Global error handlers | Centralized error management | Lost error context, hard to debug | Only for truly generic errors (network failures) |
| No session state versioning | Simpler state updates | Race conditions, UI inconsistencies | Only in single-session MVP |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| cc-sdk subprocess | Assuming process survives SIGSTOP/SIGCONT | Implement cooperative pause protocol with handshakes |
| Tauri IPC | Sending large payloads synchronously | Chunk data, use streaming, offload serialization to blocking pool |
| SQLite persistence | Using DEFERRED transactions | Always use BEGIN IMMEDIATE for writes |
| Tokio channels | Using unbounded channels for streams | Bounded channels with explicit capacity and backpressure |
| Svelte stores | Direct mutation from IPC events | Sequence numbers, optimistic updates, reconciliation |
| cc-sdk ClaudeSDKClient | Sharing instances between sessions | One client per session, never pool or share |
| Process lifecycle | Forgetting to reap child processes | Register SIGCHLD handlers, use Tokio's automatic reaping |
| Approval workflow | Holding locks while awaiting approval | Message-passing architecture, release locks before await |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Unbounded stream buffering | Memory growing linearly, eventual OOM | Bounded channels with backpressure, rate limiting | >1GB output per session |
| Synchronous JSON serialization | Task poll times >10ms, runtime stalls | Spawn blocking for large payloads, chunk data | Payloads >1MB |
| No SQLite connection pooling | SQLITE_BUSY under load, write timeouts | Connection pool with write queue serialization | >3 concurrent writers |
| Polling for session state | CPU waste, battery drain on laptops | Event-driven updates via Tauri events | >5 active sessions |
| No output truncation | Infinite memory for long sessions | Circular buffers, LRU eviction, pagination | Sessions running >1 hour |
| Checkpoint interference | WAL file growing unbounded | Configure checkpoint intervals, limit concurrent readers | >10 concurrent sessions |
| No stream throttling | UI overwhelmed, frame drops | Rate limit streams to 60 updates/sec per session | Verbose AI output |
| Holding locks across await | Deadlocks, head-of-line blocking | Message-passing, lock-free data structures | Any contention |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Passing unsanitized user input to cc-sdk | Command injection, prompt injection attacks | Sanitize inputs, use cc-sdk's parameter validation |
| Storing API keys in SQLite plaintext | Key exposure if database is compromised | Use OS keychain (keyring crate), encrypt at rest |
| No approval timeout | Malicious session holds resources indefinitely | 5-minute timeout with default deny |
| Executing cc-sdk with elevated privileges | Subprocess escape escalates to root | Run cc-sdk processes with minimal privileges |
| Logging full cc-sdk responses | PII/sensitive data in logs | Redact sensitive fields, limit log retention |
| No rate limiting on session spawns | Resource exhaustion DoS | Limit to 5 concurrent sessions, queue additional |
| Frontend directly controlling process signals | Privilege escalation via Tauri commands | Validate all signals, whitelist allowed operations |
| Trusting IPC event sources | Malicious webview could send fake events | Validate event signatures, use Tauri's security context |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| No indication of session pause/resume | User confused why session isn't responding | Clear "paused" badge, explanation text, resume button |
| Losing scroll position on updates | Jarring UX during streaming output | Detect if user is at bottom, only auto-scroll if so |
| No optimistic UI updates | Feels laggy, unresponsive to clicks | Immediately show UI changes, reconcile with server state |
| Approval dialog blocking all sessions | Other sessions appear frozen | Per-session approval, non-modal notifications |
| No session naming/labeling | Can't distinguish between 5 sessions | User-provided names, auto-generated descriptions |
| Abrupt session termination | Lost work, no chance to save | Confirmation dialog, auto-save draft state |
| No progress indication | User thinks app is frozen | Spinners, progress bars, "typing" indicators |
| Hidden errors | Silent failures, broken trust | Toast notifications, inline error messages, retry buttons |

## "Looks Done But Isn't" Checklist

- [ ] **Session Pause/Resume:** Often missing state persistence before pause — verify session can be killed and restarted from persisted state
- [ ] **Approval Workflow:** Often missing timeout and cancel handling — verify approval dialogs have 5-min timeout and explicit cancel
- [ ] **Multi-Session Output:** Often missing output isolation — verify Session A's stdout never appears in Session B's UI
- [ ] **Process Cleanup:** Often missing zombie reaping — verify `ps aux | grep defunct` shows 0 zombies after 100 session cycles
- [ ] **Error Recovery:** Often missing graceful degradation — verify session crashes don't crash entire app
- [ ] **SQLite Concurrency:** Often missing write serialization — verify 5 concurrent writes don't cause SQLITE_BUSY
- [ ] **Memory Bounds:** Often missing channel capacity limits — verify app memory stays <500MB after 1000 messages per session
- [ ] **State Synchronization:** Often missing version/sequence numbers — verify rapid concurrent updates display correctly
- [ ] **IPC Backpressure:** Often missing flow control — verify fast producers don't OOM slow consumers
- [ ] **Window Lifecycle:** Often missing session persistence on close — verify sessions resume correctly after window close/reopen

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| SQLite write deadlock | LOW | Enable WAL, change to BEGIN IMMEDIATE, add retry logic |
| Unbounded channel OOM | MEDIUM | Add bounded channels, implement backpressure, restart affected sessions |
| IPC serialization blocking | MEDIUM | Identify large payloads, chunk them, offload to blocking pool |
| Approval workflow deadlock | HIGH | Kill deadlocked session, release locks, implement timeout |
| Zombie process accumulation | LOW | Kill zombies manually, restart app, add SIGCHLD handler |
| SIGSTOP corruption | HIGH | Kill corrupted session, restore from last checkpoint, use cooperative pause |
| Frontend state desync | LOW | Full state resync from backend, clear stale UI state |
| cc-sdk API breakage | HIGH | Pin version, implement adapter layer, migrate to new API incrementally |
| ClaudeSDKClient cross-contamination | MEDIUM | Restart all sessions, verify client isolation, add session tagging |
| Window lifecycle mismatch | LOW | Show confirmation dialog, add minimize-to-tray, implement session persistence |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| SQLite write serialization | Phase 1: Core Architecture | Stress test 10 concurrent writes, 0 SQLITE_BUSY errors |
| Unbounded channel OOM | Phase 1: Core Architecture | Monitor memory over 1-hour session, <500MB total |
| IPC serialization blocking | Phase 1: Core Architecture | Measure task poll times, all <10ms |
| Approval workflow deadlock | Phase 2: Approval System | Test 5 pending approvals, no deadlocks |
| Zombie process accumulation | Phase 1: Core Architecture | Run 100 session cycles, 0 zombie processes |
| SIGSTOP corruption | Phase 2: Session Lifecycle | Pause/resume 50 times, 0 session crashes |
| Frontend state desync | Phase 3: Multi-Session UI | Send 1000 concurrent updates, UI always correct |
| cc-sdk API breakage | Phase 1: Core Architecture | Integration tests fail on API changes |
| ClaudeSDKClient cross-contamination | Phase 1: Core Architecture | Run 5 sessions with distinct queries, verify output isolation |
| Window lifecycle mismatch | Phase 3: Multi-Session UI | Close window with active sessions, verify graceful handling |

## Sources

### Tauri v2 Architecture
- [Inter-Process Communication | Tauri](https://v2.tauri.app/concept/inter-process-communication/)
- [Process Model | Tauri](https://v2.tauri.app/concept/process-model/)
- [Tauri Architecture | Tauri](https://v2.tauri.app/concept/architecture/)

### SQLite Concurrency
- [How Turso Eliminates SQLite's Single-Writer Bottleneck | Better Stack Community](https://betterstack.com/community/guides/databases/turso-explained/)
- [SQLite concurrent writes and "database is locked" errors](https://tenthousandmeters.com/blog/sqlite-concurrent-writes-and-database-is-locked-errors/)
- [Beyond the Single-Writer Limitation with Turso's Concurrent Writes](https://turso.tech/blog/beyond-the-single-writer-limitation-with-tursos-concurrent-writes)
- [Write-Ahead Logging](https://sqlite.org/wal.html)

### Rust Async & Concurrency
- [Spawning | Tokio - An asynchronous Rust runtime](https://tokio.rs/tokio/tutorial/spawning)
- [Channels | Tokio - An asynchronous Rust runtime](https://tokio.rs/tokio/tutorial/channels)
- [How to Use async Rust Without Blocking the Runtime](https://oneuptime.com/blog/post/2026-01-07-rust-async-without-blocking/view)
- [The Hidden Bottleneck: Blocking in Async Rust](https://cong-or.xyz/blocking-async-rust)
- [Rust Async Just Killed Your Throughput and You Didn't Notice | Medium](https://medium.com/@shkmonty35/rust-async-just-killed-your-throughput-and-you-didnt-notice-c38dd119aae5)

### Channel Backpressure
- [Rust concurrency: a streaming workflow, served with a side of back-pressure](https://medium.com/@polyglot_factotum/rust-concurrency-a-streaming-workflow-served-with-a-side-of-back-pressure-955bdf0266b5)
- [How to Build a High-Throughput Data Ingestion Pipeline in Rust](https://oneuptime.com/blog/post/2026-01-25-high-throughput-data-ingestion-pipeline-rust/view)
- [Bounded or Unbounded? Rust mpsc vs Go Channels Explained | Medium](https://medium.com/@sonampatel_97163/bounded-or-unbounded-rust-mpsc-vs-go-channels-explained-658aaae57b57)

### Process Management
- [The guide to signal handling in Rust - LogRocket Blog](https://blog.logrocket.com/guide-signal-handling-rust/)
- [How to Build a Graceful Shutdown Handler in Rust](https://oneuptime.com/blog/post/2026-01-07-rust-graceful-shutdown/view)
- [Signal handling - Command Line Applications in Rust](https://rust-cli.github.io/book/in-depth/signals.html)

### cc-sdk
- [cc-sdk - Rust SDK for Claude Code CLI](https://crates.io/crates/cc-sdk)
- [cc_sdk - Rust](https://docs.rs/cc-sdk/latest/cc_sdk/)

### State Management
- [State management • SvelteKit Docs](https://svelte.dev/docs/kit/state-management)
- [Week 7: How to manage shared state in Svelte 5 with runes | Medium](https://medium.com/@chose/week-7-how-to-manage-shared-state-in-svelte-5-with-runes-77a4ad305b8a)
- [How to Create Session Persistence](https://oneuptime.com/blog/post/2026-01-30-session-persistence/view)

---
*Pitfalls research for: Tauri v2 + cc-sdk Multi-Session AI Agent Orchestrator*
*Researched: 2026-02-14*
