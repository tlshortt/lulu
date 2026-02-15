# Debug Session: App shell white window

## Symptoms
- Truth: Tauri app opens to a dark UI shell with a left sidebar labeled "Lulu" and "Mission Control", a "New Session" button at the bottom, and a main area showing the "No active sessions" empty state with the ⌘ + N hint.
- Expected: Same as truth.
- Actual: Blank white window. The OS title bar renders. It's briefly dark (blinks) and then goes to white.
- Errors: None reported.
- Reproduction: Test 1 in UAT (launch app shell).
- Timeline: Discovered during UAT.

## Notes
- Goal: find_root_cause_only.
