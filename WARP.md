# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

This is a Rust-based HTTP service that provides REST API endpoints for controlling media playback and system volume on Linux. The service acts as a bridge between external HTTP clients and Linux MPRIS-compatible media players, while also presenting itself as an MPRIS service ("My Player") to desktop environments.

## Architecture

### Core Components

- **HTTP Server**: Built with Actix Web, serves REST endpoints on `0.0.0.0:8080`
- **MPRIS Integration**: 
  - **Publisher**: Uses `souvlaki` crate to advertise itself as "My Player" on D-Bus
  - **Client**: Uses `mpris` crate to discover and control external media players
- **System Volume Control**: Uses PulseAudio's `pactl` command for system volume adjustment
- **Authentication**: Bearer token middleware for all endpoints

### Key Dependencies

- `actix-web`: HTTP server framework
- `souvlaki`: MPRIS service publishing (D-Bus integration)
- `mpris`: MPRIS client for controlling external players
- `serde`/`serde_json`: JSON serialization
- `enigo`: System input simulation (unused in current implementation)

### Application State

The service maintains shared state with:
- `MediaControls`: MPRIS publisher instance
- `MediaMetadata`: Last set metadata (title, artist, album)
- `MediaPlayback`: Current playback state (Playing/Paused)

All state is wrapped in `Arc<Mutex<>>` for thread-safe access across HTTP handlers.

## Development Commands

### Building and Running

```bash
# Development build
cargo build

# Release build  
cargo build --release

# Run development server with API token
MEDIA_CONTROL_API_TOKEN="your-secret-token" cargo run

# Run release binary with custom bind address
MEDIA_CONTROL_API_TOKEN="your-secret-token" ./target/release/media-controller
```

### Testing and Development

```bash
# Check dependencies
cargo tree

# Format code
cargo fmt

# Lint code
cargo clippy

# Check without building
cargo check

# Test API endpoints (requires running service)
curl -X POST http://localhost:8080/play \
  -H "Authorization: Bearer your-secret-token"

curl -X GET http://localhost:8080/status \
  -H "Authorization: Bearer your-secret-token"

# Test with custom preferred player
MEDIA_CONTROL_PREFERRED_PLAYER="firefox" \
MEDIA_CONTROL_API_TOKEN="your-secret-token" cargo run
```

### Installation

```bash
# Install system-wide
sudo install -m 755 target/release/media-controller /usr/local/bin/media-controller

# Install from crates.io (package name differs)
cargo install media-control-server
```

## Configuration

### Required Environment Variable

- `MEDIA_CONTROL_API_TOKEN`: Bearer token for API authentication (must be set)

### Optional Environment Variables

- `MEDIA_CONTROL_PREFERRED_PLAYER`: Preferred MPRIS player to control (default: "chromium")
  - Examples: "chromium", "firefox", "spotify", "vlc"
  - Case-insensitive substring matching (e.g., "chrome" matches "chromium")

### Service Configuration

The service is designed to run as a systemd service (user or system-wide). See README.md for systemd service file examples.

### Network Configuration

- Default bind: `0.0.0.0:8080` (hardcoded in `main.rs:104`)
- No TLS/SSL (plain HTTP)
- Firewall must allow port 8080 for external access

## API Endpoints

All endpoints require `Authorization: Bearer <token>` header.

**Control Endpoints** (POST):
- `/play`, `/pause`, `/toggle` - Playback control
- `/next`, `/previous` - Track navigation  
- `/seek_forward`, `/seek_backward` - 30-second seeking
- `/volume_up`, `/volume_down` - System volume (5% increments)

**Status Endpoint** (GET):
- `/status` - Returns current state, metadata, and which player is being controlled

## Key Implementation Details

### MPRIS Integration Strategy

The service implements a dual MPRIS approach:
1. **Publisher**: Creates "My Player" service visible to desktop environments
2. **Client**: Discovers and controls the first external player (excluding itself)

This allows the service to appear as a unified media player while proxying commands to actual players.

### Player Discovery

`find_player()` function uses intelligent player selection:
1. **Priority**: Looks for the preferred player (default: Chromium/Chrome)
2. **Fallback**: Uses the first available MPRIS player (excluding "My Player")
3. **Logging**: Prints which player is selected and why

This solves the issue where MPRIS stack ordering changes between boots, ensuring consistent control of your preferred browser/player.

### Volume Control

System volume is controlled via PulseAudio's `pactl` command:
```bash
pactl set-sink-volume @DEFAULT_SINK@ +5%
pactl set-sink-volume @DEFAULT_SINK@ -5%
```

### Authentication Middleware

Custom Actix Web middleware validates Bearer tokens on all requests before reaching handlers. Invalid/missing tokens return 401 Unauthorized.

## Platform Requirements

- Linux (tested on Manjaro, Ubuntu)
- Rust 1.60+
- PulseAudio with `pactl` available
- D-Bus (for MPRIS integration)
- Systemd (for service management)

## Development Notes

- Single source file architecture (`src/main.rs`) - consider modularization for growth
- No automated tests - integration testing requires external MPRIS players
- Hardcoded values (port 8080, seek duration 30s, volume increment 5%) could be configurable
- Error handling is basic - most operations use `let _ = ...` to ignore failures
- Player prioritization system ensures consistent Chromium/Chrome targeting regardless of MPRIS stack ordering
