# Feature Research

**Domain:** AI Coding Agent Orchestrator
**Researched:** 2026-02-14
**Confidence:** HIGH

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete.

| Feature | Why Expected | Complexity | Dependencies | Notes |
|---------|--------------|------------|--------------|-------|
| **Multiple parallel sessions** | Core value prop - run 3-5 agents simultaneously | MEDIUM | Git worktrees, process isolation | Every orchestrator supports this. CodeLayer, Devin, Windsurf, Conductor all emphasize parallel execution |
| **Session status monitoring** | Need to know if agents are working, idle, blocked, or errored | LOW | Process lifecycle tracking | Essential for "at a glance" monitoring. Devin, Antigravity, Noveum all provide status indicators |
| **Git worktree integration** | Prevent agents from stepping on each other's work | MEDIUM | Git worktrees API | Industry standard practice in 2026 for parallel agents. Each session = separate worktree |
| **Session naming/labeling** | Distinguish "auth refactor" from "add tests" from "fix bug #123" | LOW | Session metadata storage | Basic UX requirement for managing multiple sessions |
| **Streaming output** | See what agents are doing in real-time | MEDIUM | WebSocket/SSE, terminal emulation | Users need visibility into agent progress. All terminal-based tools provide this |
| **Approval prompts for dangerous ops** | User approval before file deletion, git push, destructive commands | HIGH | Action classification, approval rules engine | Critical security feature. Claude Code, Codex, Devin all require approval for risky actions |
| **Session pause/resume** | Interrupt agent to review plan or halt bad direction | MEDIUM | Process lifecycle, state serialization | Devin 2.1 added explicit "wait for approval" on low confidence plans |
| **Session history/logs** | Review what agent did, debug issues, learn from work | LOW | Append-only log storage | CodeLayer emphasizes "comprehensive activity logs" for team learning |
| **Basic error handling** | Graceful failure when agent hits errors, not crash entire app | MEDIUM | Process supervision, error boundaries | Table stakes for production reliability |
| **Cross-session isolation** | One crashed session doesn't kill others | MEDIUM | Process sandboxing | Basic requirement for parallel execution |

### Differentiators (Competitive Advantage)

Features that set the product apart. Not expected, but valuable.

| Feature | Value Proposition | Complexity | Dependencies | Notes |
|---------|-------------------|------------|--------------|-------|
| **Auto-approve rules** | Skip approval for safe, repetitive actions based on patterns | HIGH | Rule engine, action classification, pattern matching | NOT table stakes yet - most tools require manual approval. Big UX win for "approve npm install every time" scenarios |
| **Dashboard list view** | See all sessions, their status, branches at a glance | LOW | UI framework, data binding | Lulu's core differentiator. Most tools focus on single session UIs (Cursor, Windsurf, Aider) |
| **Session templates/presets** | Quick-start common patterns: "Add tests", "Fix linter", "Implement ticket" | MEDIUM | Template storage, prompt engineering | Makes orchestrator more accessible. Bolt v2 has "Claude Agent" presets |
| **Cross-session coordination** | Lead agent spawns sub-agents, merges results | VERY HIGH | Agent SDK, IPC, merge conflict resolution | Claude Code's agent teams, Gas Town's Mayor/Polecats model. Ambitious feature |
| **Visual diff review** | Side-by-side diff UI before committing changes | MEDIUM | Diff rendering, syntax highlighting | Claude Desktop, Cursor, Devin Review emphasize visual diff. Huge UX improvement over terminal |
| **Session teleport/handoff** | Start on web/mobile, continue in desktop app | HIGH | Cloud sync, session serialization, cross-platform | Claude Code's /teleport and /desktop commands. Strong mobile-to-desktop flow |
| **Confidence scoring** | Agent indicates likelihood of task completion | MEDIUM | Model output parsing, confidence calibration | Devin 2.1 added this. Helps users prioritize review time |
| **Automated testing verification** | Agent runs tests, iterates until passing | MEDIUM | Test runner integration, retry logic | Aider, Bolt v2 emphasize this. Reduces error rate significantly |
| **Memory/learning system** | Agents remember coding style, patterns, APIs across sessions | HIGH | Vector DB, context retrieval, prompt engineering | Windsurf's Memory, Claude Code's memory preview. Competitive differentiator |
| **Bulk operations** | Apply same task across multiple worktrees (e.g., "fix lint in all feature branches") | MEDIUM | Multi-session orchestration, result aggregation | Enables scaling to 10+ parallel sessions |
| **Session cost tracking** | Token usage and $ cost per session | LOW | API metering, cost calculation | Braintrust, Noveum emphasize this. Important for teams managing budgets |
| **Slack/chat integration** | Route tasks from team chat -> PR | MEDIUM | Slack API, webhook handlers | Claude Code's @Claude in Slack. Expands beyond desktop app use case |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **Real-time collaboration on same session** | "Let's pair program with AI together" | Merge conflicts, unclear who's in control, degraded UX | Each person runs their own session, share results via PR |
| **Unlimited parallel sessions** | "Why cap at 5? Let me run 50!" | 1) API rate limits, 2) User can't monitor 50 sessions, 3) Merge conflicts explode | Cap at 5-10 sessions, focus on quality not quantity |
| **Fully autonomous mode (zero approval)** | "Let AI run unsupervised overnight" | Security nightmare, cost explosion, low-quality output. Research shows HITL is critical | Smart auto-approve rules for known-safe operations only |
| **Agent-to-agent chat UI** | "Watch agents talk to each other" | Misleading anthropomorphism, no actionable value | Show hierarchical task breakdown and progress bars |
| **Session recording/replay** | "Record and replay exact agent sessions" | 1) Non-deterministic LLM output, 2) Expensive, 3) Privacy concerns | Activity logs with commands executed, not full replay |
| **Built-in code review AI** | "AI reviews other AI's code" | Adds latency, cost, and false confidence. Human review > AI review | Show visual diffs clearly, make human review fast |
| **Session priorities/scheduling** | "High priority sessions run first" | Adds complexity, users expect parallel = parallel | Run all sessions in parallel, user manually stops low-priority ones if needed |
| **Cross-repo orchestration** | "Run agents across 10 different repos" | 1) Explosion in complexity, 2) Unclear merge strategy, 3) Rare use case | Focus on single-repo multi-worktree case, document multi-repo workarounds |

## Feature Dependencies

```
Session Management Core (required for everything)
    ├──> Multiple Parallel Sessions (table stakes)
    │       ├──> Git Worktree Integration (table stakes)
    │       ├──> Session Naming/Labeling (table stakes)
    │       ├──> Cross-Session Isolation (table stakes)
    │       └──> Dashboard List View (differentiator)
    │
    ├──> Session Lifecycle
    │       ├──> Session Status Monitoring (table stakes)
    │       ├──> Streaming Output (table stakes)
    │       ├──> Session Pause/Resume (table stakes)
    │       └──> Session History/Logs (table stakes)
    │
    ├──> Security & Approvals
    │       ├──> Approval Prompts (table stakes)
    │       └──> Auto-Approve Rules (differentiator, requires approval system)
    │
    └──> Advanced Features (all depend on core being solid)
            ├──> Visual Diff Review (differentiator)
            ├──> Confidence Scoring (differentiator)
            ├──> Session Templates (differentiator)
            ├──> Session Cost Tracking (differentiator)
            └──> Memory/Learning System (differentiator)

Cross-Session Coordination (complex, depends on everything above)
    └──> Agent Teams, Sub-agents, Merge orchestration
```

### Critical Dependencies

- **Git worktrees before parallel sessions**: Can't safely run parallel agents without worktrees
- **Approval system before auto-approve rules**: Need baseline approval flow first
- **Session lifecycle before advanced features**: Pause/resume/history must work before adding visual diff, etc.
- **Core stability before cross-session coordination**: Most complex feature, requires everything else working

### Feature Conflicts

- **Auto-approve rules vs Security**: Easy to make too permissive, need conservative defaults
- **Streaming output vs Performance**: Real-time updates add UI overhead, need efficient rendering
- **Memory system vs Privacy**: Persisting coding patterns raises data retention questions

## MVP Definition

### Launch With (v0.1 - Greenfield milestone)

Minimum viable product focused on core orchestration value.

- [x] **Multiple parallel sessions (3-5)** - Core value prop, validates the product concept
- [x] **Dashboard list view** - Main differentiator vs single-session tools
- [x] **Session status monitoring** - Essential for "at a glance" awareness
- [x] **Streaming output per session** - Users need to see what agents are doing
- [x] **Session naming** - Basic UX requirement for distinguishing sessions
- [x] **Approval prompts for dangerous ops** - Security table stakes
- [x] **Session pause/resume** - Users need control to interrupt bad paths
- [x] **Git worktree integration** - Prevents agents from conflicting
- [x] **Basic error handling** - Must not crash entire app if one session fails
- [x] **Session history/logs** - Debug and review what happened

**MVP = "Conductor but native and Rust-based"**

### Add After Validation (v0.2-v0.5)

Features to add once core is working and users validate the approach.

- [ ] **Auto-approve rules** (v0.2) - Big UX win, wait until approval system is proven
- [ ] **Visual diff review** (v0.2) - Major UX improvement over terminal diffs
- [ ] **Session templates/presets** (v0.3) - Makes tool more accessible, needs user research first
- [ ] **Session cost tracking** (v0.3) - Important for teams, straightforward addition
- [ ] **Confidence scoring** (v0.4) - Helps prioritize review time
- [ ] **Automated testing verification** (v0.4) - Reduces error rate
- [ ] **Bulk operations** (v0.5) - Scales to 10+ sessions

### Future Consideration (v1.0+)

Features to defer until product-market fit is established.

- [ ] **Memory/learning system** - Complex, requires vector DB infrastructure
- [ ] **Cross-session coordination** - Very complex, unclear if users want this
- [ ] **Session teleport/handoff** - Requires cloud backend, mobile app
- [ ] **Slack/chat integration** - Expands beyond desktop app, significant scope
- [ ] **Advanced merge orchestration** - Only needed if cross-session coordination ships

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority | Rationale |
|---------|------------|---------------------|----------|-----------|
| Multiple parallel sessions | **CRITICAL** | MEDIUM | **P0** | Core value prop, product doesn't work without this |
| Dashboard list view | **CRITICAL** | LOW | **P0** | Main UI differentiator |
| Session status monitoring | **CRITICAL** | LOW | **P0** | Essential for usability |
| Streaming output | **CRITICAL** | MEDIUM | **P0** | Users need real-time visibility |
| Approval prompts | **CRITICAL** | HIGH | **P0** | Security requirement |
| Git worktree integration | **CRITICAL** | MEDIUM | **P0** | Prevents catastrophic conflicts |
| Session naming | HIGH | LOW | **P0** | Basic UX requirement |
| Session pause/resume | HIGH | MEDIUM | **P0** | Users need control |
| Session history/logs | HIGH | LOW | **P0** | Debug and review capability |
| Cross-session isolation | **CRITICAL** | MEDIUM | **P0** | Reliability requirement |
| Auto-approve rules | **HIGH** | HIGH | **P1** | Major UX improvement, wait until approval system stable |
| Visual diff review | HIGH | MEDIUM | **P1** | Big UX win, adds complexity |
| Session templates | MEDIUM | MEDIUM | **P1** | Makes tool more accessible |
| Session cost tracking | MEDIUM | LOW | **P1** | Easy win for team use cases |
| Confidence scoring | MEDIUM | MEDIUM | **P2** | Nice to have, not critical |
| Automated testing | MEDIUM | MEDIUM | **P2** | Quality improvement, not blocking |
| Bulk operations | MEDIUM | MEDIUM | **P2** | Enables scale, niche use case |
| Memory/learning | HIGH | **VERY HIGH** | **P3** | Competitive feature but huge scope |
| Cross-session coordination | LOW | **VERY HIGH** | **P3** | Unclear if users want this, very complex |
| Session teleport | LOW | **VERY HIGH** | **P3** | Requires cloud backend, mobile app |
| Slack integration | LOW | MEDIUM | **P3** | Expands beyond core use case |

**Priority key:**
- **P0: Must have for MVP** - Product doesn't work without these
- **P1: Should have post-MVP** - Add after validating core concept
- **P2: Nice to have** - Add when resources allow
- **P3: Future consideration** - Defer until product-market fit

## Competitor Feature Analysis

| Feature | CodeLayer (HumanLayer) | Cursor | Windsurf | Claude Code CLI | Devin | Lulu's Approach |
|---------|------------------------|--------|----------|-----------------|-------|-----------------|
| **Parallel sessions** | Yes (MULTICLAUDC) | No (single session) | No (single session) | No (single session) | Yes (multiple Devins) | **Yes (3-5 sessions)** |
| **Dashboard view** | Yes (desktop IDE) | No (in-editor) | No (in-editor) | No (terminal) | Yes (web dashboard) | **Yes (native app)** |
| **Session status** | Yes | N/A | N/A | N/A | Yes (confidence scores) | **Yes (at-a-glance)** |
| **Approval prompts** | Yes | Yes | Yes | Yes (--yolo flag) | Yes (low confidence waits) | **Yes (with auto-approve rules)** |
| **Git worktrees** | Yes (emphasized) | No | No | No | Yes (isolated workspaces) | **Yes (per session)** |
| **Visual diff review** | Yes | Yes (inline diffs) | Yes (inline diffs) | No (terminal) | Yes (Devin Review) | **Yes (side-by-side)** |
| **Agent teams** | No | No | No | Yes (research preview) | No | **No (future)** |
| **Memory system** | No | No | Yes (persistent memory) | Yes (research preview) | No | **No (future)** |
| **Cloud sync** | Partial | No | No | Yes (/teleport) | Yes (cloud workspace) | **No (local-first)** |
| **Cost tracking** | No | No | Indirect (credits) | No | No | **Yes (planned P1)** |
| **Auto-approve rules** | No | No | No | Flags only (--yolo) | Confidence-based | **Yes (planned P1)** |

### Key Insights

**Lulu's Niche:**
- **CodeLayer + Devin approach**: Parallel sessions, dashboard monitoring
- **But simpler**: No cloud backend, no agent teams complexity, no memory system (v0.1)
- **Native performance**: Built in Rust, not Electron/web-based
- **Better UX**: Auto-approve rules, visual diff review from day one (P1)

**What Lulu is NOT:**
- NOT an AI IDE (Cursor/Windsurf) - we orchestrate Claude Code sessions, not replace them
- NOT a cloud platform (Devin) - local-first, user's machine and API keys
- NOT an agent framework (LangChain/CrewAI) - focused on Claude Code orchestration specifically

## Feature Complexity Analysis

### Low Complexity (1-2 weeks)
- Session naming/labeling
- Session status monitoring (basic states: idle, working, blocked, error)
- Session history/logs (append-only text logs)
- Dashboard list view (basic UI with session list)
- Session cost tracking (API metering)

### Medium Complexity (3-6 weeks)
- Multiple parallel sessions (process management, IPC)
- Streaming output (terminal emulation, WebSocket/SSE)
- Git worktree integration (git worktree API, cleanup)
- Session pause/resume (process signals, state serialization)
- Cross-session isolation (process sandboxing, error boundaries)
- Visual diff review (diff rendering, syntax highlighting)
- Session templates (template storage, variable substitution)
- Bulk operations (multi-session coordination, result aggregation)
- Automated testing verification (test runner integration, retry logic)
- Confidence scoring (model output parsing)

### High Complexity (2-3 months)
- Approval prompts for dangerous ops (action classification, rule engine, UI flow)
- Auto-approve rules (pattern matching, rule storage, security considerations)
- Session teleport/handoff (cloud sync, session serialization, cross-platform)
- Slack/chat integration (webhook handlers, auth, message parsing)

### Very High Complexity (3-6 months)
- Memory/learning system (vector DB, embedding generation, context retrieval)
- Cross-session coordination (agent SDK, IPC, merge conflict resolution, orchestration logic)

## Implementation Sequencing

### Phase 0: Foundation (Week 1-4)
Core session management without Claude Code integration yet.

1. Process lifecycle management (spawn, monitor, kill)
2. Session naming and basic metadata
3. Dashboard UI with list view
4. Session status monitoring (hardcoded states)

**Risk mitigation:** Build orchestration layer before integrating Claude Code complexity.

### Phase 1: Claude Code Integration (Week 5-8)
Connect to real Claude Code sessions.

1. Claude Code CLI integration (spawn claude processes)
2. Streaming output capture (stdout/stderr piping)
3. Cross-session isolation (separate worktrees)
4. Git worktree creation/cleanup

**Risk mitigation:** Start with read-only monitoring before write operations.

### Phase 2: Control & Approval (Week 9-14)
Add user control over sessions.

1. Session pause/resume (send signals to claude process)
2. Approval prompt detection (parse claude output)
3. Approval UI flow (user clicks approve/reject in dashboard)
4. Session history/logs (persist output)
5. Basic error handling (crashed session recovery)

**Risk mitigation:** Build approval system incrementally, starting with detection only.

### Phase 3: Polish & UX (Week 15-20)
Make it production-ready.

1. Visual diff review (diff rendering component)
2. Session templates (predefined prompts)
3. Session cost tracking (API metering)
4. Auto-approve rules (Phase 1: simple allow-list)

**Risk mitigation:** These features enhance UX but aren't blocking for core functionality.

### Future Phases (v0.2+)
1. Auto-approve rules (Phase 2: pattern matching)
2. Bulk operations
3. Confidence scoring
4. Memory/learning system (requires research)
5. Cross-session coordination (requires agent SDK)

## Research Confidence Assessment

| Category | Confidence | Reasoning |
|----------|------------|-----------|
| **Table stakes features** | HIGH | Based on direct analysis of CodeLayer, Cursor, Windsurf, Claude Code CLI, Devin docs and demos. Clear consensus on what's expected. |
| **Differentiators** | MEDIUM-HIGH | Strong evidence for auto-approve rules, dashboard view, visual diff. Confidence scoring and memory systems backed by recent releases (Devin 2.1, Windsurf memory). |
| **Anti-features** | MEDIUM | Based on security research (OWASP AI Agent Security, human-in-the-loop limitations), architectural complexity discussions, and best practices articles. Some extrapolation from single-agent tools to orchestrator context. |
| **Complexity estimates** | MEDIUM | Based on similar Rust projects and integration complexity. Git worktree API is well-documented. Approval system complexity based on security requirements. Actual implementation may reveal surprises. |
| **Dependencies** | HIGH | Clear technical dependencies (can't do auto-approve without approval system, can't run parallel agents safely without worktrees). |

## Sources

### Primary Sources (Official Documentation)
- [Claude Code overview](https://code.claude.com/docs/en/overview) - Official Claude Code CLI capabilities
- [HumanLayer website](https://www.humanlayer.dev/) - CodeLayer orchestration features
- [HumanLayer GitHub](https://github.com/humanlayer/humanlayer) - Open source implementation details
- [Devin Release Notes](https://docs.devin.ai/release-notes/overview) - Devin 2.1 confidence scoring, Devin Review
- [Windsurf Editor](https://windsurf.com/) - Cascade agent, Memory system
- [Cursor IDE](https://cursor.sh/) - Composer mode, Agent mode, inline diffs

### Research Articles (2026)
- [Git Worktrees for Parallel AI Coding](https://medium.com/@mabd.dev/git-worktrees-the-secret-weapon-for-running-multiple-ai-coding-agents-in-parallel-e9046451eb96) - Industry standard practice
- [Human-in-the-Loop for AI Agents](https://www.permit.io/blog/human-in-the-loop-for-ai-agents-best-practices-frameworks-use-cases-and-demo) - Approval best practices
- [AI Agent Observability Tools](https://research.aimultiple.com/agentic-monitoring/) - Monitoring and dashboard patterns
- [OWASP AI Agent Security Top 10 2026](https://medium.com/@oracle_43885/owasps-ai-agent-security-top-10-agent-security-risks-2026-fc5c435e86eb) - Security risks and mitigation
- [AI Coding Agents: Coherence Through Orchestration](https://mikemason.ca/writing/ai-coding-agents-jan-2026/) - Architectural patterns

### Tool Comparisons
- [Top 7 AI Coding Agents 2026](https://www.lindy.ai/blog/ai-coding-agents) - Comprehensive feature comparison
- [AI Dev Tool Power Rankings](https://blog.logrocket.com/ai-dev-tool-power-rankings) - Market landscape analysis
- [Cursor Review 2026](https://www.nxcode.io/resources/news/cursor-review-2026) - Deep dive on Cursor features
- [Windsurf Review 2026](https://hackceleration.com/windsurf-review/) - Windsurf features and pricing

### Orchestration Frameworks
- [Top AI Agent Orchestration Platforms 2026](https://redis.io/blog/ai-agent-orchestration-platforms/) - Orchestration patterns
- [LLM Orchestration 2026](https://research.aimultiple.com/llm-orchestration/) - Framework comparison
- [AI Agent Frameworks 2026](https://www.vellum.ai/blog/top-ai-agent-frameworks-for-developers) - Developer frameworks

---

*Feature research for: Lulu AI Coding Agent Orchestrator*
*Researched: 2026-02-14*
