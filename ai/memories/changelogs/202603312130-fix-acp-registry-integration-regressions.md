## 2026-03-31 21:30: fix: ACP registry integration regressions

**What changed:**
- Fixed the Tauri `get_registry_agents` response to return camelCase pagination keys expected by the frontend schema.
- Updated the registry Zod schema to accept nullable optional fields like `website`, `iconUrl`, `preferredMethod`, and `installedVersion`.
- Reworked `useRegistryAgents` to use stable refs for page and query state so search, refresh, and pagination no longer race against stale closures.
- Wired the registry install button to the `install_registry_agent` command and added per-agent install loading state.
- Merged built-in and registry runtimes into one available-runtimes grid while preserving the installed-runtimes section.
- Updated the merged search UX so built-ins and registry results both respond to the same query, with debounced registry fetches.
- Replaced the backend `unimplemented!()` NPX registry install path with a normal error result.

**Why:**
- The new registry UI could not load because the backend response shape did not match the frontend validator.
- The registry payload uses explicit `null` values for some optional fields, which the frontend schema previously rejected.
- Search and pagination were resetting unexpectedly due to hook callback recreation and stale state reads.
- The merged runtime view needed one consistent search flow instead of immediate local filtering for one source and submit-only search for the other.
- The registry install button was not functional.
- Built-in runtime installation needed to stay visible alongside registry results instead of living in a separate section.
- Unsupported install methods should fail gracefully instead of panicking the app.

**Files affected:**
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `crates/peekoo-agent-app/src/agent_provider_service.rs`
- `apps/desktop-ui/src/hooks/useRegistryAgents.ts`
- `apps/desktop-ui/src/features/agent-runtimes/AgentProviderPanel.tsx`
- `apps/desktop-ui/src/features/agent-runtimes/RegistryAgentCard.tsx`
