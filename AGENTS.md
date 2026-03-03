# AGENTS.md - Peekoo AI Development Guidelines

## Build, Test, and Lint Commands

### Rust (Backend)
```bash
# Run all tests
just test
# Run single test (filter by name)
cargo test <test_name>

# Check code without building
just check

# Format all Rust code
just fmt

# Lint with Clippy
just lint

# Build for production
just build

# Clean build artifacts
just clean
```

### TypeScript/React (Frontend)
```bash
# Install dependencies
cd apps/desktop-ui && bun install

# Run dev server
bun run dev

# Build for production
bun run build

# Type check
npx tsc --noEmit
```

## Code Style Guidelines

## Single Responsibility Principle (SRP)

- Keep each function focused on one reason to change.
- Keep each module/crate focused on one core concern.
- When a file starts mixing transport, persistence, and business rules, split it into focused modules.
- Prefer composition over large all-in-one services.
- Refactors should improve boundaries first, then behavior.
- New features should follow existing boundaries unless there is a clear design reason to introduce a new one.

### Rust
- **Naming**: `snake_case` for functions/variables, `PascalCase` for types/traits, `SCREAMING_SNAKE_CASE` for constants
- **Error Handling**: Use `thiserror` for custom error types, prefer `Result<T, E>` over panics
- **Types**: Be explicit with types, avoid `impl Trait` in public APIs unless necessary
- **Documentation**: Document public APIs with `///` doc comments
- **Testing**: Use `#[cfg(test)]` modules, write unit tests for domain logic

### TypeScript/React
- **Naming**: `camelCase` for functions/variables, `PascalCase` for components/types, `SCREAMING_SNAKE_CASE` for constants
- **Imports**: Group imports: React → third-party → @/ aliases → relative
- **Types**: Prefer explicit types over `any`, use Zod for runtime validation
- **Components**: Use functional components, hooks for state logic
- **Styling**: Use Tailwind CSS classes, avoid inline styles

### Project Structure
- `crates/`: Rust workspace crates
  - `core-domain/`: Shared domain models and invariants
  - `peekoo-agent/`: Agent runtime facade over the underlying SDK/session
  - `peekoo-agent-auth/`: OAuth and provider auth protocol orchestration
  - `peekoo-agent-app/`: Agent application orchestration and settings use cases
  - Other crates for plugins, persistence, security, and integrations
- `apps/desktop-ui/`: React + Vite + TypeScript frontend
- `apps/desktop-tauri/src-tauri/`: Canonical Tauri desktop runtime crate

### Agent-Centric Architecture
- Dependency flow: `desktop-ui` -> `desktop-tauri` -> `peekoo-agent-app` -> (`peekoo-agent`, `peekoo-agent-auth`, `persistence-sqlite`, `security`).
- `desktop-tauri` is a transport layer only; avoid embedding persistence, OAuth protocol, or runtime orchestration logic in command handlers.
- `peekoo-agent` owns prompt/session runtime concerns; it must not depend on UI or Tauri crates.
- `peekoo-agent-auth` owns OAuth/provider auth flow concerns only.

### Deprecations
- `core-app` has been removed and must not be reintroduced.
- New application orchestration should live in domain-specific app crates (current: `peekoo-agent-app`).

### Git Conventions
- Sign commits with GPG
- Use conventional commits format: `type(scope): subject`
- Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`
- Keep commits atomic and focused

## Common Tasks

### Running the Desktop App
```bash
# Start dev mode (runs both frontend and Tauri)
just dev
```

### Adding a New Crate
1. Create `crates/<name>/` with `Cargo.toml` and `src/lib.rs`
2. Add to workspace `Cargo.toml` members list
3. Follow naming convention: `peekoo-<name>`

### Running Single Tests
```bash
# Run tests matching pattern
cargo test <pattern>

# Example: run only pomodoro tests
cargo test pomodoro
```

## AI Knowledge Base

This repository includes an AI memory system in the `ai/` directory. See `ai/AGENTS.md` for details on:
- **Changelogs**: Record significant changes after completing work
- **Diagrams**: Architecture and flow documentation using Mermaid
- **Plans**: Implementation plans for features

**Quick Reference:**
| Action | Location | When |
|--------|----------|------|
| Write changelog | `ai/memories/changelogs` | After significant changes |
| Create diagram | `ai/memories/diagrams/<name>.md` | When documenting architecture |
| Save plan | `ai/plans/<feature>.md` | After planning phase |
| Load memories/plans | `ai/` | When context needed |
