# Continuous Integration Pipeline

This document explains the comprehensive CI/CD pipeline for the media-controller project, which runs on every commit to ensure code quality, security, and functionality.

## 🚀 Overview

The CI pipeline runs on:
- **Push** to `main` or `develop` branches
- **Pull requests** targeting `main` or `develop` branches

The pipeline includes 7 parallel jobs that provide comprehensive testing and security analysis.

## 📋 CI Jobs

### 1. **Test & Build** (`test`)
**Duration**: ~3-5 minutes  
**Purpose**: Core functionality verification

**What it does:**
- ✅ Installs all system dependencies (D-Bus, X11, etc.)
- ✅ Checks code formatting with `cargo fmt`
- ✅ Runs comprehensive linting with `cargo clippy`
- ✅ Builds both debug and release binaries
- ✅ Runs unit tests with D-Bus session
- ✅ Runs integration tests (if any exist)
- ✅ Tests binary startup and basic functionality

**Key Features:**
- Sets up D-Bus session for MPRIS testing
- Tests with mock environment variables
- Validates binary can start without errors
- Full Rust backtrace on failures

### 2. **Security Audit** (`security-audit`)
**Duration**: ~2-3 minutes  
**Purpose**: Vulnerability detection in dependencies

**What it does:**
- 🔍 Scans all dependencies for known security vulnerabilities
- 🔍 Uses RustSec Advisory Database
- 🔍 Fails CI on any known vulnerabilities
- 🔍 Provides detailed vulnerability reports

**Tools:**
- `cargo-audit`: Official Rust security scanner
- RustSec database: Comprehensive vulnerability tracking

### 3. **Dependency Review** (`dependency-review`)
**Duration**: ~1-2 minutes  
**Purpose**: PR-specific dependency security analysis
**Trigger**: Only on Pull Requests

**What it does:**
- 📊 Reviews dependency changes in PRs
- 📊 Identifies new vulnerabilities introduced
- 📊 Checks for malicious packages
- 📊 Fails on moderate+ severity issues

### 4. **Trivy Security Scan** (`trivy-scan`)
**Duration**: ~2-3 minutes  
**Purpose**: Comprehensive vulnerability scanning

**What it does:**
- 🛡️ Scans filesystem for vulnerabilities
- 🛡️ Analyzes configuration files
- 🛡️ Uploads results to GitHub Security tab
- 🛡️ Fails on CRITICAL/HIGH severity issues

**Features:**
- SARIF output for GitHub integration
- Security tab visualization
- Multi-format reporting (table + SARIF)

### 5. **Supply Chain Security** (`supply-chain-security`)
**Duration**: ~2-4 minutes  
**Purpose**: Advanced dependency analysis

**What it does:**
- 🔗 Checks for security advisories
- 🔗 Validates license compliance
- 🔗 Detects banned/problematic dependencies
- 🔗 Analyzes dependency graph

**Tools:**
- `cargo-deny`: Comprehensive dependency analyzer
- Custom `deny.toml` configuration
- License compliance checking

### 6. **Code Quality Analysis** (`code-quality`)
**Duration**: ~4-6 minutes  
**Purpose**: Test coverage and quality metrics

**What it does:**
- 📈 Generates test coverage reports
- 📈 Uploads coverage to Codecov
- 📈 Analyzes code quality metrics
- 📈 Tracks coverage trends over time

**Tools:**
- `cargo-llvm-cov`: LLVM-based coverage tool
- Codecov integration
- D-Bus session for complete testing

### 7. **MSRV Check** (`msrv-check`)
**Duration**: ~3-4 minutes  
**Purpose**: Minimum Supported Rust Version validation

**What it does:**
- 🦀 Tests compilation with Rust 1.60 (project MSRV)
- 🦀 Ensures backward compatibility
- 🦀 Validates dependency compatibility
- 🦀 Runs tests on minimum Rust version

### 8. **Cross-platform** (`cross-platform`)
**Duration**: ~5-8 minutes  
**Purpose**: Multi-OS compatibility testing

**What it does:**
- 🖥️ Tests on Ubuntu Linux
- 🖥️ Tests on macOS
- 🖥️ Validates system dependency installation
- 🖥️ Ensures cross-platform builds work

**Platform-specific:**
- **Linux**: Full D-Bus and X11 integration
- **macOS**: Homebrew dependency management

## 🔧 System Dependencies

All jobs install required system libraries:

**Linux (Ubuntu):**
```bash
libdbus-1-dev pkg-config build-essential 
libudev-dev libx11-dev libxtst-dev libxkbcommon-dev 
dbus dbus-x11
```

**macOS:**
```bash
dbus pkg-config
```

## 🛡️ Security Features

### Comprehensive Scanning
- **Vulnerability Detection**: cargo-audit + Trivy
- **License Compliance**: Automated license checking
- **Dependency Review**: PR-level security analysis
- **Supply Chain**: Comprehensive dependency graph analysis

### GitHub Integration
- **Security Tab**: Automated SARIF uploads
- **Dependency Graph**: Visual dependency tracking
- **Alerts**: Automated security notifications
- **Reviews**: PR-level dependency impact analysis

## 📊 Quality Metrics

### Coverage Tracking
- **Tool**: cargo-llvm-cov with LLVM backend
- **Upload**: Automatic Codecov integration
- **Trends**: Historical coverage tracking
- **Reports**: Detailed line-by-line coverage

### Code Quality
- **Formatting**: cargo fmt enforcement
- **Linting**: cargo clippy with warnings-as-errors
- **Standards**: Rust community best practices
- **Compatibility**: MSRV compliance testing

## 🚫 Failure Scenarios

The CI will **fail** if:

### Code Quality Issues
- ❌ Code not formatted (`cargo fmt --check`)
- ❌ Clippy warnings present
- ❌ Tests fail on any supported platform
- ❌ Binary compilation fails

### Security Issues  
- ❌ Known vulnerabilities in dependencies
- ❌ CRITICAL/HIGH Trivy scan findings
- ❌ Banned licenses detected
- ❌ Malicious dependencies introduced

### Compatibility Issues
- ❌ MSRV (Rust 1.60) compilation fails
- ❌ Cross-platform build failures
- ❌ System dependency issues

## 🎯 Best Practices

### For Developers
```bash
# Run locally before pushing
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo build --release

# Security checks
cargo audit
cargo deny check

# Coverage analysis
cargo llvm-cov --html
```

### For Contributors
- Ensure PRs pass all CI checks
- Add tests for new functionality
- Update documentation for API changes
- Follow semantic commit conventions
- Keep dependencies up-to-date

## 📈 Performance Optimization

### Caching Strategy
- **Cargo Registry**: Shared across jobs
- **Build Artifacts**: Platform-specific caching
- **Dependencies**: Cached by Cargo.lock hash
- **Tools**: Cached tool installations

### Parallel Execution
- All jobs run in parallel when possible
- Independent security scans
- Platform-specific builds run concurrently
- Coverage and quality jobs are isolated

## 🔄 Maintenance

### Regular Updates
- Security databases auto-update
- Tool versions managed via Dependabot
- Rust toolchain stays current with stable
- System dependencies updated in CI images

### Monitoring
- GitHub Actions dashboard
- Security tab monitoring
- Codecov dashboard tracking
- Dependency graph reviews

## 🆘 Troubleshooting

### Common Issues

**D-Bus Session Failures:**
```bash
# CI solution
export $(dbus-launch)
```

**System Dependency Missing:**
- Check CI job logs for apt-get/brew failures
- Verify dependency names are current
- Test locally with same OS version

**Security Scan False Positives:**
- Review Trivy/audit reports carefully
- Add exceptions to `deny.toml` if needed
- Document security decisions in PR

**Coverage Collection Issues:**
- Ensure D-Bus session is active
- Check LLVM tools are installed
- Verify test execution environment

This comprehensive CI pipeline ensures that every commit maintains high standards for security, quality, and functionality across all supported platforms.