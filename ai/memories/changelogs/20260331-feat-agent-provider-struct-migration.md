# AgentProvider Struct Migration & Registry Source of Truth

**Date:** 2026-03-31
**Scope:** Backend architecture, provider management, ACP registry integration

## Summary

Migrated `AgentProvider` from an enum to a struct to support dynamic provider configurations from the ACP registry. This fixes the long-standing issue where registry-installed agents (like cursor) would incorrectly spawn as opencode in chat sessions.

## Changes

### Core Architecture (`crates/peekoo-agent/src/config.rs`)

- **Replaced `AgentProvider` enum with struct:**
  - Before: `AgentProvider::Opencode`, `AgentProvider::PiAcp`, etc.
  - After: `AgentProvider { id, command, args, source }`
  - Added `ProviderSource` enum: `Builtin`, `Registry`, `Custom`

- **Added factory functions:**
  - `AgentProvider::opencode()` - built-in opencode
  - `AgentProvider::pi_acp()` - built-in pi-acp
  - `AgentProvider::claude_code()` - built-in claude-code
  - `AgentProvider::codex()` - built-in codex
  - `AgentProvider::from_registry(id, command, args)` - registry-installed
  - `AgentProvider::custom(id, command, args)` - user-defined

- **Updated methods:**
  - `command()` - returns `(command, args)` from struct fields
  - `command_with_environment()` - special handling for opencode bundled path
  - `id()` - returns provider identifier
  - Added `is_builtin()`, `is_registry()`, `is_custom()` helpers

### Provider Service (`crates/peekoo-agent-app/src/agent_provider_service.rs`)

- **Extended `ProviderInfo` struct:**
  - Added `command: String` field
  - Added `args: Vec<String>` field
  - Now reads command/args from database for all providers

- **Updated SQL queries:**
  - All queries now include `command` and `args_json` columns
  - Proper deserialization of args from JSON

- **Removed legacy seeding:**
  - Deleted `seed_builtin_providers()` function
  - Deleted `upsert_runtime_record()` helper
  - Removed `is_builtin_runtime()` guard
  - Added `seed_installed_opencode()` for conditional opencode seeding

- **Fixed `remove_custom_provider`:**
  - Now checks `is_bundled` from database instead of hardcoded list
  - Allows removing registry-installed agents

### Settings Integration (`crates/peekoo-agent-app/src/settings/mod.rs`)

- **Changed `to_agent_config` signature:**
  - Before: `to_agent_config(config, provider_id: &str, model_id)`
  - After: `to_agent_config(config, provider: AgentProvider, model_id)`
  - Removed `provider_id_to_enum()` function

- **Updated all callers:**
  - Tests now use factory functions: `AgentProvider::pi_acp()`, etc.

### Application Layer (`crates/peekoo-agent-app/src/application.rs`)

- **Updated `create_agent_service`:**
  - Builds `AgentProvider` from runtime info
  - Uses factory functions for built-in providers
  - Uses `from_registry()` for registry-installed providers

- **Updated `agent_launch_env`:**
  - Same provider building logic for environment variables

### Database Migration (`crates/persistence-sqlite/migrations/`)

- **New migration `202604010001_registry_source_of_truth.sql`:**
  - Deletes uninstalled non-bundled rows (old hardcoded seeds)
  - Sets `registry_id = 'opencode'` for existing opencode rows
  - Drops unused columns:
    - `is_enabled`
    - `install_hint`
    - `registry_source`
    - `registry_metadata`
    - `last_registry_sync`
  - Drops index on `registry_source` before dropping column

### Binary Installation (`crates/acp-registry-client/src/install.rs`)

- **Added permission handling:**
  - `make_executable()` - platform-specific chmod (Unix only)
  - `make_binaries_executable()` - walks directory, sets +x on executables
  - Called after archive extraction in `install_binary()`

- **Platform-specific logic:**
  - Unix: Sets 0o755 on shell scripts and binaries
  - Windows: No-op (executables work by extension)

### Archive Extraction (`crates/peekoo-node-runtime/src/archive.rs`)

- **Preserves execute permissions:**
  - `extract_targz()`: reads mode from tar header, applies if executable
  - `extract_zip()`: reads unix_mode from zip entry, applies if executable
  - Only on Unix platforms (cfg-gated)

### ACP Agent (`crates/peekoo-agent-acp/src/agent.rs`)

- **Updated environment variable parsing:**
  - Uses factory functions: `AgentProvider::opencode()`, etc.
  - Supports all built-in providers

### Frontend Updates

- **Cleaned up `AgentProviderPanel.tsx`:**
  - Removed `filteredAvailableProviders` rendering (DB no longer has uninstalled agents)
  - Available section now shows only registry agents

- **Updated `ProviderInfo` type:**
  - Added `command` and `args` fields

## Bug Fixes

1. **Provider mismatch in chat:** Registry-installed agents (cursor, pi-acp) now correctly spawn their own backend instead of falling back to opencode
2. **Missing execute permissions:** Binary installations now properly chmod executables after extraction
3. **Duplicate DB rows:** Migration removes old hardcoded seeds that were never installed
4. **Unused columns:** Cleaned up 5 unused database columns

## Testing

- All 298 tests pass
- Updated tests to use new struct-based API:
  - `AgentProvider::pi_acp()` instead of `AgentProvider::PiAcp`
  - `provider.id()` comparisons instead of enum equality

## Backward Compatibility

- **Database:** No breaking changes - only stored provider_id strings
- **Sessions:** Existing sessions continue to work (provider_id unchanged)
- **Settings:** Settings store still uses provider_id strings
- **API:** Tauri commands unchanged

## Migration Guide

No action needed for end users. The migration runs automatically on next app start.

For developers:
- Use factory functions instead of enum variants
- Pass full `AgentProvider` struct to `to_agent_config()`
- Check `provider.source` for provider type instead of matching enum variants

## Files Changed

- `crates/peekoo-agent/src/config.rs` - Core struct definition
- `crates/peekoo-agent-app/src/agent_provider_service.rs` - Provider management
- `crates/peekoo-agent-app/src/settings/mod.rs` - Settings integration
- `crates/peekoo-agent-app/src/application.rs` - Service creation
- `crates/acp-registry-client/src/install.rs` - Binary installation
- `crates/peekoo-node-runtime/src/archive.rs` - Archive extraction
- `crates/peekoo-agent-acp/src/agent.rs` - ACP agent
- `crates/persistence-sqlite/migrations/202604010001_registry_source_of_truth.sql` - DB migration
- Frontend components - Cleaned up available providers rendering

## Related Issues

- Fixed: Chat panel showing "opencode/big-pickle" when cursor was selected as default runtime
- Fixed: NPX installation failing due to missing execute permissions
- Fixed: Database bloat from unused columns and ghost rows
