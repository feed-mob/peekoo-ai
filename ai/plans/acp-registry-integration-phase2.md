# Plan: ACP Registry Integration - Phase 2 (Full Integration)

## Overview
Full integration of ACP (Agent Client Protocol) registry with the existing agent system. Replace hardcoded "Available Runtimes" with dynamic registry showing 40+ agents, with custom ordering, search, and pagination.

## Current State
✅ **Phase 1 Complete:** `acp-registry-client` crate with fetch, cache, platform detection, installation
🔄 **Phase 2 In Progress:** Integration with AgentProviderService, Tauri commands, UI updates

## Goals
- [ ] Show all 40+ ACP registry agents in "Available Runtimes"
- [ ] Custom ordering: built-ins (opencode, pi, codex, claude) at top
- [ ] Search functionality for agents
- [ ] Pagination support (20 agents per page, "Load more")
- [ ] Install Cursor (binary) as proof of concept
- [ ] Platform-aware filtering (show only compatible agents)

## Architecture

### Backend Integration
```
AgentProviderService
├── Existing: Built-in agents (opencode, pi, claude-code, codex)
├── NEW: RegistryClient integration
│   ├── fetch_registry_agents() - Get from CDN/cache
│   ├── search_registry_agents() - Filter by query
│   ├── install_registry_agent() - Install Cursor/Gemini/etc
│   └── refresh_registry() - Force refresh
└── NEW: Database columns for registry tracking
```

### Frontend Integration
```
AgentProviderPanel
├── NEW: Search bar at top
├── NEW: RegistryAgentCard components
├── UPDATED: Available Runtimes section
│   ├── Shows 40+ agents from registry
│   ├── Custom ordering (built-ins first)
│   ├── Platform compatibility badges
│   └── Install buttons
└── NEW: Infinite scroll / pagination
```

## Implementation Steps

### Phase 2.1: Database Schema Update

**Migration:** `202603311800_add_registry_columns.sql`

```sql
-- Add registry source tracking to agent_runtimes
ALTER TABLE agent_runtimes ADD COLUMN registry_source TEXT; -- "builtin", "acp_registry", "custom"
ALTER TABLE agent_runtimes ADD COLUMN registry_id TEXT;      -- "gemini", "cursor" from ACP registry
ALTER TABLE agent_runtimes ADD COLUMN registry_version TEXT; -- Version from registry
ALTER TABLE agent_runtimes ADD COLUMN registry_metadata TEXT; -- JSON: authors, license, website, icon_url
ALTER TABLE agent_runtimes ADD COLUMN last_registry_sync TEXT; -- ISO timestamp
```

### Phase 2.2: AgentProviderService Refactoring

**File:** `crates/peekoo-agent-app/src/agent_provider_service.rs`

#### 2.2.1 Add RegistryClient to Service

```rust
pub struct AgentProviderService {
    // ... existing fields
    registry_client: acp_registry_client::RegistryClient,
}
```

#### 2.2.2 New Data Structures

```rust
/// Registry agent info for UI
#[derive(Debug, Clone)]
pub struct RegistryAgentInfo {
    pub registry_id: String,          // "gemini", "cursor"
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
    pub license: String,
    pub website: Option<String>,
    pub icon_url: Option<String>,
    pub supported_platforms: Vec<String>,
    pub supported_methods: Vec<InstallationMethod>,
    pub is_supported_on_current_platform: bool,
    pub preferred_method: Option<InstallationMethod>,
    pub is_installed: bool,
    pub installed_version: Option<String>,
    pub display_order: i32,           // For custom ordering
}

/// Filter options for registry agents
#[derive(Debug, Clone, Default)]
pub struct RegistryFilterOptions {
    pub search_query: Option<String>,
    pub platform_only: bool,
    pub method_filter: Option<InstallationMethod>,
    pub sort_by: RegistrySortBy,
    pub page: usize,                  // 1-based
    pub page_size: usize,
}

pub enum RegistrySortBy {
    Featured,     // Custom order: built-ins first
    Name,
    PlatformSupport,
}
```

#### 2.2.3 Custom Ordering Logic

**Priority (display_order):**
1. **Featured built-ins:** opencode(0), pi-acp(1), codex-acp(2), claude-acp(3)
2. **Popular agents:** gemini(4), cursor(5), goose(6), kimi(7), qwen-code(8), cline(9), auggie(10)
3. **Alphabetical** for rest

```rust
fn calculate_display_order(registry_id: &str) -> i32 {
    match registry_id {
        "opencode" => 0,
        "pi-acp" => 1,
        "codex-acp" => 2,
        "claude-acp" => 3,
        "gemini" => 4,
        "cursor" => 5,
        "goose" => 6,
        "kimi" => 7,
        "qwen-code" => 8,
        "cline" => 9,
        "auggie" => 10,
        _ => 100 + (registry_id.chars().next().unwrap_or('z') as i32),
    }
}
```

#### 2.2.4 New Methods

```rust
impl AgentProviderService {
    pub async fn fetch_registry_agents(
        &self,
        filter: &RegistryFilterOptions,
    ) -> anyhow::Result<(Vec<RegistryAgentInfo>, usize)>; // (agents, total_count)
    
    pub async fn search_registry_agents(
        &self,
        query: &str,
    ) -> anyhow::Result<Vec<RegistryAgentInfo>>;
    
    pub async fn install_registry_agent(
        &self,
        registry_id: &str,
        method: InstallationMethod,
    ) -> anyhow::Result<InstallProviderResponse>;
    
    pub async fn refresh_registry(&self) -> anyhow::Result<()>;
}
```

### Phase 2.3: Tauri Commands

**File:** `apps/desktop-tauri/src-tauri/src/lib.rs`

Add 5 new commands:

```rust
#[tauri::command]
async fn get_registry_agents(
    filter: RegistryFilterOptionsDto,
    state: State<'_, AgentState>,
) -> Result<PaginatedRegistryAgentsDto, String>;

#[tauri::command]
async fn search_registry_agents(
    query: String,
    state: State<'_, AgentState>,
) -> Result<Vec<RegistryAgentDto>, String>;

#[tauri::command]
async fn install_registry_agent(
    registry_id: String,
    method: String,
    state: State<'_, AgentState>,
) -> Result<InstallProviderResponseDto, String>;

#[tauri::command]
async fn refresh_registry_catalog(
    state: State<'_, AgentState>,
) -> Result<(), String>;

#[tauri::command]
async fn get_registry_agent_details(
    registry_id: String,
    state: State<'_, AgentState>,
) -> Result<RegistryAgentDto, String>;
```

### Phase 2.4: Frontend UI

#### 2.4.1 New Types

**File:** `apps/desktop-ui/src/types/agent-registry.ts`

```typescript
export interface RegistryAgent {
  registryId: string;
  name: string;
  version: string;
  description: string;
  authors: string[];
  license: string;
  website?: string;
  iconUrl?: string;
  supportedMethods: InstallationMethod[];
  isSupported: boolean;
  preferredMethod?: InstallationMethod;
  isInstalled: boolean;
  installedVersion?: string;
}

export interface PaginatedRegistryAgents {
  agents: RegistryAgent[];
  totalCount: number;
  page: number;
  pageSize: number;
  hasMore: boolean;
}
```

#### 2.4.2 New Hook

**File:** `apps/desktop-ui/src/hooks/useRegistryAgents.ts`

```typescript
export function useRegistryAgents() {
  const [agents, setAgents] = useState<RegistryAgent[]>([]);
  const [loading, setLoading] = useState(false);
  const [hasMore, setHasMore] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  
  const fetchAgents = useCallback(async (reset = false) => { ... });
  const search = useCallback((query: string) => { ... });
  const loadMore = useCallback(() => { ... });
  
  return { agents, loading, hasMore, search, loadMore, refresh };
}
```

#### 2.4.3 UI Components

**File:** `apps/desktop-ui/src/features/agent-runtimes/RegistryAgentCard.tsx`

- Show agent icon, name, description
- Display supported methods (NPX, Binary badges)
- Platform compatibility indicator
- Install button (disabled if not supported)

**Update:** `AgentProviderPanel.tsx`

- Add search bar at top
- Replace hardcoded "Available Runtimes" with registry list
- Add infinite scroll / "Load more" button
- Custom ordering (built-ins at top)

### Phase 2.5: Binary Installation (Cursor)

**Test Case:** Install Cursor agent

- Registry ID: "cursor"
- Method: Binary
- Download: Platform-specific archive
- Extraction: To `~/.peekoo/resources/agents/cursor/`
- Verification: Check executable exists

## Files to Modify/Create

### Backend (Rust)
1. **Migration:** `crates/persistence-sqlite/migrations/202603311800_add_registry_columns.sql`
2. **Service:** `crates/peekoo-agent-app/src/agent_provider_service.rs` - Add registry integration
3. **Tauri:** `apps/desktop-tauri/src-tauri/src/lib.rs` - Add 5 new commands
4. **Cargo:** `crates/peekoo-agent-app/Cargo.toml` - Add acp-registry-client dependency

### Frontend (TypeScript/React)
5. **Types:** `apps/desktop-ui/src/types/agent-registry.ts` - New type definitions
6. **Hook:** `apps/desktop-ui/src/hooks/useRegistryAgents.ts` - Data fetching hook
7. **Component:** `apps/desktop-ui/src/features/agent-runtimes/RegistryAgentCard.tsx` - Agent card
8. **Update:** `apps/desktop-ui/src/features/agent-runtimes/AgentProviderPanel.tsx` - Main panel

## Testing Checklist

### Backend Tests
- [ ] Registry fetch returns 40+ agents
- [ ] Platform filtering works (show only compatible)
- [ ] Custom ordering: built-ins first
- [ ] Search filters by name/description
- [ ] Pagination returns correct slices
- [ ] Binary installation downloads and extracts

### Integration Tests
- [ ] Install Cursor (binary) successfully
- [ ] Cursor appears in "Installed Runtimes"
- [ ] Can run ACP commands with Cursor
- [ ] Search for "gemini" returns correct results
- [ ] Load more pagination works

### Manual Tests
- [ ] Browse all 40+ agents in UI
- [ ] Built-ins (opencode, pi, codex, claude) show at top
- [ ] Platform badges show correctly
- [ ] Install button disabled for unsupported platforms
- [ ] Offline mode uses cached registry

## Success Criteria

- [x] **Phase 1:** acp-registry-client crate complete
- [ ] **Phase 2.1:** Database migration applied
- [ ] **Phase 2.2:** AgentProviderService refactored
- [ ] **Phase 2.3:** Tauri commands added
- [ ] **Phase 2.4:** Frontend shows 40+ agents
- [ ] **Phase 2.5:** Cursor installation works
- [ ] **Feature:** Search functionality
- [ ] **Feature:** Pagination (Load more)
- [ ] **Feature:** Custom ordering (built-ins first)
- [ ] **Feature:** Platform filtering

## Timeline Estimate

- **Phase 2.1** (Database): 2 hours
- **Phase 2.2** (Backend): 4-6 hours
- **Phase 2.3** (Tauri): 2 hours
- **Phase 2.4** (Frontend): 4-6 hours
- **Phase 2.5** (Testing/Cursor): 2-4 hours

**Total: 14-20 hours** for full Phase 2 integration

## Dependencies

Already satisfied:
- ✅ `acp-registry-client` crate
- ✅ `peekoo-node-runtime` crate
- ✅ Existing Tauri infrastructure
- ✅ Existing React/TypeScript frontend

## Next Steps

1. Create database migration
2. Update AgentProviderService
3. Add Tauri commands
4. Create frontend types and hook
5. Build UI components
6. Test Cursor installation

**Ready to implement!** 🚀
