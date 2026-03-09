# Plugin Store Architecture

Data flow for plugin discovery, installation, and uninstallation from the GitHub-based plugin store.

```mermaid
graph TB
    subgraph Frontend["Frontend (desktop-ui)"]
        PM[PluginManagerPanel]
        PS[PluginStoreCatalog]
        PL[PluginList]
        HPS[usePluginStore hook]
        HP[usePlugins hook]
    end

    subgraph Tauri["Tauri Layer"]
        CMD_CATALOG[plugin_store_catalog]
        CMD_INSTALL[plugin_store_install]
        CMD_UNINSTALL[plugin_store_uninstall]
        CMD_LIST[plugins_list]
    end

    subgraph App["App Layer (peekoo-agent-app)"]
        AA[AgentApplication]
        PSS[PluginStoreService]
        PR[PluginRegistry]
    end

    subgraph Store["Plugin Store Crate"]
        DTO[StorePluginDto]
        ENUM[PluginSource enum]
        FETCH[fetch_catalog]
        DL[download_plugin_files]
        UNINST[uninstall_plugin]
    end

    subgraph External["External"]
        GH[GitHub API<br/>api.github.com]
        RAW[GitHub Raw<br/>raw.githubusercontent.com]
        FS[~/.peekoo/plugins/]
    end

    PM -->|Installed tab| PL
    PM -->|Store tab| PS
    PS --> HPS
    PL --> HP

    HPS -->|fetch| CMD_CATALOG
    HPS -->|install| CMD_INSTALL
    HPS -->|uninstall| CMD_UNINSTALL
    HP -->|list| CMD_LIST

    CMD_CATALOG --> AA
    CMD_INSTALL --> AA
    CMD_UNINSTALL --> AA
    CMD_LIST --> AA

    AA --> PSS
    AA --> PR

    PSS --> FETCH
    PSS --> DL
    PSS --> UNINST

    FETCH -->|GET /repos/.../contents/plugins| GH
    FETCH -->|GET raw/.../peekoo-plugin.toml| RAW
    DL -->|GET raw/.../files| RAW
    DL -->|write| FS
    UNINST -->|delete| FS

    PR -->|load from| FS
    PR -->|discover| FS

    DTO --> ENUM
    FETCH --> DTO
    DL --> PR
```

## Data Flow Summary

### Fetch Catalog
1. Frontend calls `plugin_store_catalog` on Store tab open
2. `AgentApplication.store_catalog()` → `PluginStoreService.fetch_catalog()`
3. Service fetches plugin directory list from GitHub API
4. For each plugin directory, fetches `peekoo-plugin.toml` from raw URL
5. Cross-references with local `PluginRegistry.discover()` to set `installed` and `source`
6. Returns `Vec<StorePluginDto>` to frontend

### Install Plugin
1. Frontend calls `plugin_store_install` with `pluginKey`
2. `AgentApplication.store_install()` → `PluginStoreService.install_plugin()`
3. Service creates `~/.peekoo/plugins/<key>/` directory
4. Recursively downloads all files from GitHub raw URLs
5. Calls `PluginRegistry.install_plugin()` to load the WASM
6. Returns updated `StorePluginDto` with `installed: true, source: Store`
7. Frontend updates local catalog state and refreshes installed list

### Uninstall Plugin
1. Frontend calls `plugin_store_uninstall` with `pluginKey`
2. `AgentApplication.store_uninstall()` → `PluginStoreService.uninstall_plugin()`
3. Verifies plugin exists in `~/.peekoo/plugins/<key>/`
4. Calls `PluginRegistry.unload_plugin()` to unload WASM
5. Deletes `~/.peekoo/plugins/<key>/` directory
6. Frontend updates local catalog state and refreshes installed list

## Key Design Decisions

- **Separate crate**: `peekoo-plugin-store` is isolated from `peekoo-agent-app` for clean SRP
- **Global-only install**: Only `~/.peekoo/plugins/` is used for user-facing installs; workspace `plugins/` is for development only
- **Optimistic UI**: Frontend updates catalog state locally after install/uninstall without re-fetching
- **Per-plugin loading**: `usePluginStore` tracks installing state per plugin via `Set<string>`
- **Cleanup on failure**: Partial downloads are cleaned up to avoid blocking future installs
- **Recursion limit**: Max 10 directory levels to prevent stack overflow from malicious API responses
