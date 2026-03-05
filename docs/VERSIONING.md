# Warden Version Management

## Overview

Warden uses semantic versioning with a single source of truth: the `.version` file.

## Version Synchronization

All version information is kept in sync:
- `.version` - Single source of truth
- `Cargo.toml` - Must match `.version`
- `src/main.rs` - Reads from `env!("CARGO_PKG_VERSION")`
- Git tags - Format: `v0.1.0`, `v0.2.0`, etc.

## Development Workflow

### Making Changes

1. Edit code as usual
2. Compile and install (all-in-one):
   ```bash
   ./installers/install-linux.sh --build
   ```
   Or separately:
   ```bash
   cargo build --release
   ./installers/install-linux.sh
   ```
3. Verify: `warden --version`

### Creating a Release

1. Update `.version` file:
   ```bash
   echo "0.2.0" > .version
   ```

2. Sync `Cargo.toml`:
   ```bash
   cargo build --release  # Verifies it compiles
   ```

3. Publish:
   ```bash
   ./installers/release.sh 0.2.0
   ```

4. The script handles:
   - Version validation
   - Git tag creation
   - Binary packaging
   - Checksum generation
   - Release notes

## Checking Versions

### Installed Version
```bash
warden --version
```

### Compiled Version
```bash
cat .version
```

### Check for Updates
```bash
./installers/check-updates.sh
```

## Version Format

```
MAJOR.MINOR.PATCH

0.1.0   = First release
0.1.1   = Bug fix
0.2.0   = New features
1.0.0   = Stable release
```

## Files Modified by Version System

- `.version` - Always updated by developer
- `Cargo.toml` - Auto-synced on `cargo build`
- `git tag` - Created by `release.sh`

## Validation

Before release, `release.sh` validates:
- ✓ `.version` matches `Cargo.toml`
- ✓ Project compiles successfully
- ✓ Git tag doesn't already exist
- ✓ Binary is ready for distribution

## Troubleshooting

### Version Mismatch

If `.version` and `Cargo.toml` don't match:

```bash
echo "0.1.0" > .version
sed -i 's/version = "[^"]*"/version = "0.1.0"/' Cargo.toml
cargo build --release
git add .version Cargo.toml
git commit -m "fix: sync versions"
```

### Tag Already Exists

If a tag already exists:

```bash
git tag -d v0.1.0           # Delete local tag
git push origin :v0.1.0     # Delete remote tag (if pushed)
./installers/release.sh 0.1.0
```

## Warden Subcommands

### Version
```bash
warden version
# Shows: Warden v0.1.0 and version file content
```

### Check Updates
```bash
warden check-updates
# Shows installed version and latest available
```

### Clear Cache
```bash
warden clear-cache
# Clears local analysis cache
```
