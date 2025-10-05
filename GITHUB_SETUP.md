# GitHub Actions Setup Guide

This document explains how to set up the automated release pipeline for the media-controller project.

## 🔐 Required Secrets

To enable automated publishing to crates.io, you need to set up the following GitHub repository secret:

### Setting up CARGO_REGISTRY_TOKEN

1. **Get your crates.io API token:**
   - Visit [crates.io](https://crates.io/)
   - Sign in with your GitHub account
   - Go to **Account Settings** → **API Tokens**
   - Click **Generate New Token**
   - Give it a descriptive name like "media-controller-ci"
   - Copy the generated token (you won't see it again!)

2. **Add the token to GitHub Secrets:**
   - Go to your GitHub repository
   - Click **Settings** → **Secrets and variables** → **Actions**
   - Click **New repository secret**
   - Name: `CARGO_REGISTRY_TOKEN`
   - Value: Paste your crates.io API token
   - Click **Add secret**

## 🚀 How the Workflow Works

The workflow automatically triggers when a Pull Request is **merged** into the `main` branch and:

1. **📦 System Setup:**
   - Installs required Linux packages (libdbus-dev, etc.)
   - Sets up Rust toolchain with formatting and linting tools
   - Caches dependencies for faster builds

2. **🏷️ Version Management:**
   - Analyzes commit messages to determine version bump type:
     - **Major**: Contains "breaking" or "major" 
     - **Minor**: Contains "feat", "feature", or "minor"
     - **Patch**: Everything else (default)
   - Updates `Cargo.toml` and `Cargo.lock` with new version
   - Commits version changes back to main branch

3. **✅ Quality Checks:**
   - Runs `cargo fmt --check` (code formatting)
   - Runs `cargo clippy` (linting)
   - Runs `cargo test` (tests)
   - Builds release binary

4. **🏷️ Release Creation:**
   - Creates and pushes git tag (e.g., `v0.2.0`)
   - Creates GitHub release with:
     - Auto-generated release notes
     - Installation instructions
     - Usage examples
   - Uploads compiled binary as release asset

5. **📦 crates.io Publishing:**
   - Automatically publishes to crates.io with correct version
   - Users can immediately install with `cargo install media-controller`

## 📋 Commit Message Convention

To control version bumps, use these keywords in your commit messages:

```bash
# Patch release (0.1.0 → 0.1.1)
git commit -m "Fix player selection bug"

# Minor release (0.1.0 → 0.2.0)  
git commit -m "feat: Add Firefox player support"

# Major release (0.1.0 → 1.0.0)
git commit -m "breaking: Change API endpoint structure"
```

## 🔧 Manual Workflow Trigger

The workflow only runs on **merged** pull requests to `main`. To trigger a release:

1. Create a feature branch
2. Make your changes
3. Create a Pull Request to `main`
4. Merge the Pull Request (don't just push to main directly)

## 🏗️ Local Testing

Before pushing, you can test the build locally with the same dependencies:

```bash
# Install system dependencies (Ubuntu/Debian)
sudo apt-get install libdbus-1-dev pkg-config build-essential

# Or on Arch/Manjaro  
sudo pacman -S dbus pkgconf base-devel

# Test the full build process
cargo fmt --check
cargo clippy -- -D warnings  
cargo test
cargo build --release
```

## 🐛 Troubleshooting

### Build fails with "libdbus not found"
- The workflow installs `libdbus-1-dev` and related packages
- This should resolve the `libdbus-sys` compilation issues

### Version not updating correctly
- Check that commit messages contain the right keywords
- Verify the workflow has write permissions to the repository

### crates.io publish fails
- Ensure `CARGO_REGISTRY_TOKEN` secret is set correctly
- Verify you own the crate name on crates.io
- Check that version doesn't already exist

### Workflow not triggering
- Make sure you're **merging** PRs, not pushing directly to main
- Check that the PR is actually merged, not just closed

## 📈 Release Timeline

Once a PR is merged:
- **~2-3 minutes**: Build and test
- **~1 minute**: Version bump and tagging  
- **~1 minute**: GitHub release creation
- **~2 minutes**: crates.io publishing
- **Total**: ~5-7 minutes from merge to availability

Users can install the new version immediately after the workflow completes!