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
  - `core-domain/`: Domain models and business logic
  - `core-app/`: Application use cases
  - Other crates for plugins, persistence, etc.
- `apps/desktop-ui/`: React + Vite + TypeScript frontend
- `apps/desktop-tauri/`: Tauri desktop app wrapper

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
