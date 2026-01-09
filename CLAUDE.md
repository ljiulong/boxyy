# CLAUDE.md - Boxy Project Documentation for AI Assistants

This document provides comprehensive guidance for AI assistants working with the Boxy codebase.

## Project Overview

**Boxy** is a unified package manager interface that provides CLI, TUI, and GUI tools to manage multiple package managers through a consistent interface. It's written in Rust and supports:

- **Package Managers**: brew, mas, npm, pnpm, yarn, bun, pip, pipx, uv, cargo
- **Interfaces**: CLI (command-line), TUI (terminal UI), GUI (desktop app via Tauri)
- **Platform Support**:
  - macOS: Full support (all managers, all interfaces)
  - Linux: CLI/TUI only (excludes brew, mas)
  - Windows: CLI/TUI only (excludes brew, mas)

**Current Version**: 0.6.3

## Repository Structure

```
boxyy/
├── crates/                      # Rust workspace crates
│   ├── cli/                     # CLI implementation
│   │   ├── src/
│   │   │   ├── main.rs         # CLI entry point with clap commands
│   │   │   └── managers.rs     # Manager factory functions
│   │   └── Cargo.toml
│   ├── tui/                     # Terminal UI implementation
│   │   ├── src/
│   │   │   ├── main.rs         # TUI entry point
│   │   │   ├── app.rs          # Application state
│   │   │   ├── ui/             # UI rendering (ratatui)
│   │   │   └── components/     # Reusable UI components
│   │   └── Cargo.toml
│   ├── core/                    # Core abstractions
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── manager.rs      # PackageManager trait
│   │   │   ├── package.rs      # Package model, Capability enum
│   │   │   ├── executor.rs     # Execution layer
│   │   │   └── retry.rs        # Retry logic with exponential backoff
│   │   └── Cargo.toml
│   ├── cache/                   # Caching layer
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   └── models.rs
│   │   └── Cargo.toml
│   ├── error/                   # Error types
│   │   └── src/lib.rs          # BoxyError enum
│   └── managers/                # Package manager implementations
│       ├── brew/               # Homebrew
│       ├── npm/                # npm
│       ├── pnpm/               # pnpm
│       ├── yarn/               # Yarn
│       ├── bun/                # Bun
│       ├── pip/                # pip
│       ├── pipx/               # pipx
│       ├── uv/                 # uv
│       ├── cargo/              # Cargo
│       └── mas/                # Mac App Store CLI
├── boxy-gui/                    # Tauri backend for GUI
│   ├── src/
│   │   ├── main.rs             # Tauri app entry point
│   │   └── lib.rs              # Tauri commands
│   ├── Cargo.toml
│   ├── tauri.conf.json         # Tauri configuration
│   └── icons/                  # App icons
├── gui-frontend/                # React frontend for GUI
│   ├── src/
│   │   ├── main.tsx            # React entry point
│   │   ├── App.tsx             # Main app component
│   │   ├── components/         # React components
│   │   ├── store/              # Zustand state management
│   │   ├── hooks/              # Custom React hooks
│   │   ├── lib/                # Utilities (API, i18n, logger)
│   │   └── types/              # TypeScript type definitions
│   ├── package.json
│   ├── vite.config.ts
│   └── tsconfig.json
├── scripts/                     # Build and CI scripts
│   ├── ci/
│   │   ├── package-tauri.sh    # Tauri build script
│   │   └── verify-artifacts.sh # Artifact verification
│   └── quick_test.sh
├── .github/workflows/
│   └── release-please.yml      # CI/CD pipeline
├── Formula/boxy.rb              # Homebrew formula (CLI/TUI)
├── Casks/boxy-gui.rb            # Homebrew cask (GUI)
├── Cargo.toml                   # Workspace configuration
├── release-please-config.json   # Release configuration
├── .release-please-manifest.json
├── README.md                    # User-facing documentation (Chinese)
├── CONTRIBUTING.md              # Contribution guidelines
├── CHANGELOG.md                 # Changelog (auto-generated)
└── LICENSE                      # MIT OR Apache-2.0
```

## Architecture

### Core Abstractions

#### PackageManager Trait (crates/core/src/manager.rs)

All package managers implement this async trait:

```rust
#[async_trait]
pub trait PackageManager: Send + Sync {
    fn name(&self) -> &str;
    async fn check_available(&self) -> Result<bool>;
    async fn list_installed(&self) -> Result<Vec<Package>>;
    async fn search(&self, query: &str) -> Result<Vec<Package>>;
    async fn get_info(&self, name: &str) -> Result<Package>;
    async fn install(&self, name: &str, version: Option<&str>, force: bool) -> Result<()>;
    async fn upgrade(&self, name: &str) -> Result<()>;
    async fn uninstall(&self, name: &str, force: bool) -> Result<()>;
    async fn check_outdated(&self) -> Result<Vec<Package>>;
    async fn list_dependencies(&self, name: &str) -> Result<Vec<Package>>;
    fn capabilities(&self) -> &[Capability];
    fn cache_key(&self) -> &str;
    fn supports(&self, capability: Capability) -> bool;
}
```

#### Package Model (crates/core/src/package.rs)

```rust
pub struct Package {
    pub name: String,
    pub version: String,
    pub manager: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub installed_path: Option<String>,
    pub size: Option<u64>,
    pub outdated: bool,
    pub latest_version: Option<String>,
}
```

#### Capability System

Managers declare their capabilities:

```rust
pub enum Capability {
    ListInstalled,
    SearchRemote,
    QueryDependencies,
    VersionSelection,
}
```

### Caching Strategy

- **Location**: `~/.cache/boxy/` (or platform-specific cache dir)
- **Implementation**: `crates/cache/src/lib.rs`
- **Cache Keys**: Per-manager, per-scope (e.g., `npm-global`, `npm-local-<hash>`)
- **Invalidation**: After install/upgrade/uninstall operations
- **TTL**: Configurable expiration

### Scope System

Some managers (npm, pnpm, yarn, bun) support:
- **Global scope**: System-wide packages
- **Local scope**: Project-specific packages (requires `--dir`)

## Development Workflows

### Adding a New Package Manager

1. **Create manager crate**: `crates/managers/<manager_name>/`
2. **Implement PackageManager trait**: Follow patterns in `crates/managers/npm/src/lib.rs`
3. **Add to workspace**: Update root `Cargo.toml` members list
4. **Register in CLI**: Update `crates/cli/src/managers.rs`:
   - Add to `MANAGER_NAMES` array
   - Add case in `create_manager()` function
   - If supports global/local scope, update `supports_global()`
5. **Register in TUI**: Update `crates/tui/src/managers.rs` similarly
6. **Register in GUI**: Update `boxy-gui/Cargo.toml` dependencies and `boxy-gui/src/lib.rs`
7. **Add tests**: Create `tests/` directory in manager crate

### Building the Project

```bash
# Build all CLI/TUI binaries
cargo build --release -p boxy-cli -p boxy-tui

# Build specific manager
cargo build -p boxy-npm

# Build GUI (requires Node.js/pnpm)
cd gui-frontend && pnpm install && pnpm build
cd ../boxy-gui && cargo tauri build

# Run tests
cargo test --workspace

# Run specific binary
cargo run -p boxy-cli -- scan
cargo run -p boxy-tui
```

### Testing Locally

```bash
# Test CLI
./target/release/boxy scan
./target/release/boxy list --manager npm
./target/release/boxy search ripgrep --manager brew

# Test TUI
./target/release/boxy-tui

# Test with verbose logging
./target/release/boxy --verbose scan
```

## Commit Conventions

This project uses **Conventional Commits** for automated versioning via release-please:

```
<type>: <description>

[optional body]
```

### Commit Types

- `feat`: New feature (bumps minor version)
- `fix`: Bug fix (bumps patch version)
- `docs`: Documentation only
- `chore`: Maintenance tasks
- `refactor`: Code refactoring
- `test`: Adding/updating tests
- `feat!` or `BREAKING CHANGE:`: Breaking change (bumps major version)

### Examples

```bash
git commit -m "feat: add support for bun package manager"
git commit -m "fix: handle empty npm list output"
git commit -m "docs: update installation instructions"
git commit -m "chore: update dependencies"
git commit -m "feat!: change PackageManager trait interface"
```

### Version Management

- **Current**: Defined in `Cargo.toml` workspace section
- **Synced files**: `boxy-gui/tauri.conf.json`, `gui-frontend/package.json`
- **Automation**: release-please creates PRs on `main` branch
- **Releases**: Triggered when release PR is merged

## CI/CD Pipeline

**Workflow**: `.github/workflows/release-please.yml`

### Jobs

1. **release-please**: Creates/updates release PR
2. **cli-tui**: Builds CLI/TUI for Linux, macOS, Windows
3. **gui-macos**: Builds Tauri GUI for macOS (includes signing)
4. **release**: Publishes artifacts to GitHub Releases
5. **update-brew**: Updates Homebrew formula/cask

### Build Artifacts

- **CLI/TUI**: `boxy-cli-tui-v<version>-<OS>.tar.gz|.zip`
- **GUI**: `Boxy-v<version>-macos.dmg`, `Boxy-v<version>-macos.app.zip`
- **Updater**: `*.app.tar.gz` + `.sig` for auto-updates
- **Metadata**: `latest.json` for Tauri updater

### Secrets Required

- `RELEASE_PLEASE_TOKEN`: GitHub token with write permissions
- `TAURI_PRIVATE_KEY`: Code signing key
- `TAURI_KEY_PASSWORD`: Signing key password

## Key Conventions for AI Assistants

### When Adding Features

1. **Read existing implementations first**: Check similar managers before implementing
2. **Follow async patterns**: All I/O operations use `async`/`await` with tokio
3. **Handle timeouts**: Use `COMMAND_TIMEOUT` (300s) or `READ_COMMAND_TIMEOUT` (60s)
4. **Cache appropriately**: Use `cache.set()` after fetching, `cache.invalidate()` after mutations
5. **Error handling**: Return `Result<T>` using `BoxyError` variants
6. **Logging**: Use `tracing` macros (`debug!`, `info!`, `warn!`, `error!`)

### Code Style

- **Rust**: Follow `rustfmt` defaults (run `cargo fmt`)
- **Comments**: Chinese comments are used throughout (match existing style)
- **Naming**: Snake_case for functions/variables, PascalCase for types
- **Testing**: Add unit tests in manager crates, integration tests in CLI/TUI

### Common Patterns

#### Command Execution

```rust
async fn exec(&self, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("npm");
    cmd.args(args);
    if let Some(workdir) = &self.workdir {
        cmd.current_dir(workdir);
    }
    let output = timeout(COMMAND_TIMEOUT, cmd.output())
        .await
        .map_err(|_| BoxyError::CommandTimeout)?
        .map_err(|_| BoxyError::CommandFailed { /* ... */ })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(BoxyError::CommandFailed { /* ... */ })
    }
}
```

#### JSON Parsing

```rust
let output = self.exec(&["list", "--json"]).await?;
let data: NpmListOutput = serde_json::from_str(&output)
    .map_err(|e| BoxyError::JsonError {
        message: format!("解析 npm list 输出失败: {}", e),
    })?;
```

#### Caching

```rust
async fn list_installed(&self) -> Result<Vec<Package>> {
    // Check cache first
    if let Some(cached) = self.cache.get(self.cache_key()).await? {
        return Ok(cached);
    }

    // Fetch data
    let packages = /* ... fetch logic ... */;

    // Store in cache
    self.cache.set(self.cache_key(), &packages).await?;

    Ok(packages)
}
```

### Platform-Specific Code

Use conditional compilation:

```rust
#[cfg(target_os = "macos")]
fn ensure_macos_path() {
    // macOS-specific PATH handling
}

#[cfg(target_os = "macos")]
use std::env;
```

### Error Messages

- User-facing: Use Chinese (matches README.md language)
- Internal: Can be English or Chinese
- Structured: Use `BoxyError` variants, not string errors

## Important Files to Know

### Configuration

- **Cargo.toml**: Workspace config, dependency versions, package metadata
- **release-please-config.json**: Version sync config
- **.release-please-manifest.json**: Current version number
- **boxy-gui/tauri.conf.json**: Tauri app config (identifier, bundle, updater)

### Entry Points

- **CLI**: `crates/cli/src/main.rs` (line 127: `#[tokio::main] async fn main()`)
- **TUI**: `crates/tui/src/main.rs`
- **GUI Backend**: `boxy-gui/src/main.rs`
- **GUI Frontend**: `gui-frontend/src/main.tsx`

### Manager Registry

- **CLI**: `crates/cli/src/managers.rs` (line 19: `MANAGER_NAMES`)
- **TUI**: `crates/tui/src/managers.rs`
- **GUI**: `boxy-gui/src/lib.rs`

## Common Tasks for AI Assistants

### Bug Fixes

1. Reproduce the issue locally
2. Check error logs (use `--verbose` flag)
3. Fix the bug in the appropriate crate
4. Add regression test if possible
5. Commit with `fix:` prefix

### Feature Additions

1. Understand scope (CLI only? TUI? GUI? All three?)
2. Update core abstractions if needed (`crates/core/`)
3. Implement in CLI (`crates/cli/`)
4. Implement in TUI (`crates/tui/`) - UI changes needed
5. Implement in GUI (backend + frontend)
6. Update documentation (README.md if user-facing)
7. Commit with `feat:` prefix

### Documentation Updates

1. **User docs**: Update `README.md` (in Chinese)
2. **Developer docs**: Update this file (CLAUDE.md)
3. **API docs**: Add Rust doc comments (`///`)
4. Commit with `docs:` prefix

### Debugging Tips

- **Verbose logs**: Run with `--verbose` to see debug output
- **Cache issues**: Use `--no-cache` to bypass cache
- **Timeout issues**: Check `COMMAND_TIMEOUT` and `READ_COMMAND_TIMEOUT` constants
- **Platform issues**: Test on target platform (macOS, Linux, Windows)
- **JSON parsing**: Print raw output before parsing to see format

## Frequently Asked Questions

### Why both CLI and TUI?

- **CLI**: For scripting and automation (CI/CD, shell scripts)
- **TUI**: For interactive daily use (browsing, searching, bulk operations)
- **GUI**: For users who prefer desktop apps

### Why Rust?

- Performance (fast startup, low memory)
- Safety (no segfaults, thread-safe)
- Cross-platform (single codebase for all OSes)
- Ecosystem (Tauri, clap, ratatui)

### Why separate manager crates?

- Modularity (easy to add/remove managers)
- Parallel compilation (faster builds)
- Clean dependencies (each manager only needs its parser deps)

### How does the cache work?

- Stores JSON-serialized `Vec<Package>` on disk
- Uses TTL + explicit invalidation
- Per-manager, per-scope keys
- Reduces repeated command execution (e.g., `brew list` is slow)

### Can I run this on Windows/Linux?

- **Yes** for CLI/TUI (but no brew/mas support)
- **No** for GUI (currently macOS-only, Tauri can support Windows/Linux but not built)

### How do I test GUI changes?

```bash
cd gui-frontend
pnpm install
pnpm dev              # Opens dev server with hot reload

# In another terminal
cd boxy-gui
cargo tauri dev       # Launches GUI with dev frontend
```

### How do I sign the macOS app?

- Set up Apple Developer account
- Create signing certificate
- Add `TAURI_PRIVATE_KEY` and `TAURI_KEY_PASSWORD` secrets
- CI automatically signs during `gui-macos` job

## Resources

- **Repository**: https://github.com/ljiulong/boxyy
- **Issues**: https://github.com/ljiulong/boxyy/issues
- **Releases**: https://github.com/ljiulong/boxyy/releases
- **Tauri Docs**: https://tauri.app/
- **Ratatui Docs**: https://ratatui.rs/
- **Clap Docs**: https://docs.rs/clap/

## Summary

Boxy is a well-structured Rust project with clear separation of concerns:

- **Core abstractions** define behavior (PackageManager trait)
- **Manager implementations** handle specific tools
- **Three interfaces** (CLI/TUI/GUI) share the same core logic
- **Conventional commits** drive automated releases
- **Multi-platform CI/CD** builds and distributes binaries

When working on this project:
1. Follow existing patterns (check similar code first)
2. Use async/await for all I/O
3. Handle errors with `Result<T>` and `BoxyError`
4. Cache aggressively, invalidate carefully
5. Test on target platforms
6. Write conventional commits for proper versioning

This codebase is designed for extensibility - adding a new package manager should take ~1-2 hours by following the npm/brew examples.
