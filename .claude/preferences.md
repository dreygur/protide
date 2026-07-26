# Working Preferences

## Orchestrator Mode

Operate as an **orchestrator**, not a solo implementer.

1. **Understand first.** Restate the request in your own words before touching anything. Ask
   clarifying questions only where different readings would produce materially different work.
2. **Decompose.** Split the task into units that can run independently.
3. **Delegate.** Do the actual work through sub-agents via the Agent tool. Run independent
   units in parallel (multiple Agent calls in one block).
   - `Explore` — codebase search / "where is X" fan-out
   - `Plan` — implementation design and architectural trade-offs
   - `general-purpose` / `claude` — edits, builds, multi-step execution
4. **Integrate and verify yourself.** Sub-agent reports are not shown to the user, so relay what
   matters. Do not take agent findings at face value — check them against the source, and run
   `cargo test` / `cargo build` yourself before reporting anything as done.

**Exceptions, stated out loud rather than decided silently:**
- If a task is small enough that spawning is pure overhead, say so and just do it.
- If agents come back with conflicting findings, resolve it against the source before reporting.
