## 2026-03-31 20:30: feat: Complete ACP Registry Integration - Phase 2

**What changed:**
- **Phase 2.1: Database Migration** ✅
  - Created migration to add registry columns to agent_runtimes table
  - registry_source, registry_id, registry_version, registry_metadata, last_registry_sync
  - Indexes for efficient lookups

- **Phase 2.2: Backend Integration** ✅
  - Integrated acp-registry-client into AgentProviderService
  - Added RegistryClient field with Optional handling for graceful degradation
  - Implemented fetch_registry_agents() with pagination, filtering, and custom ordering
  - Implemented search_registry_agents() for text search across registry
  - Implemented install_registry_agent() for binary installations (Cursor proof of concept)
  - Implemented refresh_registry() to force CDN refresh
  - Added custom display ordering: built-ins (0-3), popular (4-10), alphabetical (100+)
  - All 71 existing tests passing

- **Phase 2.3: Tauri Commands** ✅
  - Added 4 new Tauri commands: get_registry_agents, search_registry_agents, install_registry_agent, refresh_registry_catalog
  - Commands properly delegate to AgentProviderService
  - Error handling with user-friendly messages

- **Phase 2.4: Frontend UI** ✅
  - Created TypeScript types (RegistryAgent, PaginatedRegistryAgents) with Zod validation
  - Created useRegistryAgents hook with pagination (20 per page) and search
  - Created RegistryAgentCard component showing icon, version, description, supported methods
  - Updated AgentProviderPanel with:
    - Search bar for filtering 40+ agents
    - Registry agents grid with platform badges
    - "Load more" pagination button
    - Agent count display
    - Platform compatibility indicators (shows "Unsupported" for incompatible platforms)
  - Frontend builds successfully

**Key Features Implemented:**
- ✅ Show all 40+ ACP registry agents in "Available Runtimes"
- ✅ Custom ordering: opencode, pi-acp, codex-acp, claude-acp at top (0-3)
- ✅ Popular agents next: gemini, cursor, goose, kimi, qwen-code, cline, auggie (4-10)
- ✅ Search functionality across agent names and descriptions
- ✅ Pagination support with "Load more" button
- ✅ Platform-aware filtering (shows only compatible agents by default)
- ✅ Binary installation support (ready for Cursor test)
- ✅ Graceful offline support (uses cached registry data)

**Files Created/Modified:**
- Migration: `crates/persistence-sqlite/migrations/202603311800_add_registry_columns.sql`
- Backend: `crates/peekoo-agent-app/src/agent_provider_service.rs` (+454 lines)
- Backend: `crates/peekoo-agent-app/src/application.rs` (+registry methods)
- Backend: `crates/peekoo-agent-app/src/lib.rs` (+exports)
- Tauri: `apps/desktop-tauri/src-tauri/src/lib.rs` (+4 commands)
- Frontend: `apps/desktop-ui/src/types/agent-registry.ts` (new)
- Frontend: `apps/desktop-ui/src/hooks/useRegistryAgents.ts` (new)
- Frontend: `apps/desktop-ui/src/features/agent-runtimes/RegistryAgentCard.tsx` (new)
- Frontend: `apps/desktop-ui/src/features/agent-runtimes/AgentProviderPanel.tsx` (updated)

**Testing:**
- ✅ 71 existing tests passing
- ✅ Backend compiles with no errors
- ✅ Frontend builds successfully
- ✅ Tauri commands registered and accessible

**Next Steps (Phase 2.5 - Testing):**
- Test Cursor installation (binary download + extract)
- Verify search filters correctly
- Test pagination (Load more)
- Verify custom ordering (built-ins at top)
- Test offline mode (use cached data)

**Ready for Cursor installation test!** 🚀
