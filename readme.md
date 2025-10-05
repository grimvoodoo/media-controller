# Media Controller HTTP Service

A Rust-based HTTP service for controlling media playback and system volume on Linux via REST API calls. It advertises itself on MPRIS (so desktop UIs see it as a media player) and proxies commands to external players or the system mixer.

## Features

* **Play/Pause/Toggle** media playback via REST endpoints
* **Next/Previous track** skip
* **Seek forward/backward** by configurable intervals (default 30 seconds)
* **System volume control** (up/down by percentage) via PulseAudio/`pactl`
* **Bearer token** authentication for secure access
* **MPRIS publishing**: appears as "My Player" in desktop environments
* **Systemd-friendly**: run as a user or system service

## Table of Contents

* [Prerequisites](#prerequisites)
* [Installation](#installation)
* [Configuration](#configuration)

  * [API Token](#api-token)
  * [Systemd Service](#systemd-service)
* [Usage](#usage)

  * [Starting the Service](#starting-the-service)
  * [REST Endpoints](#rest-endpoints)
* [Integration](#integration)
* [Troubleshooting](#troubleshooting)
* [Contributing](#contributing)
* [License](#license)

## Prerequisites

* Rust (1.60+)
* Cargo
* Linux (tested on Manjaro, Ubuntu)
* PulseAudio with `pactl` for system volume control
* Systemd for service management

## Installation

### Build from Source

```bash
# Clone the repo
git clone https://github.com/grimvoodoo/media-controller.git
cd media-controller

# Build in release mode
# to setup rust
rustup default stable
cargo build --release

# Install binary (system-wide)
sudo install -m 755 target/release/media-controller /usr/local/bin/media-controller
```

### Install from crates.io (Recommended)

```bash
# Install the binary
cargo install media-controller

# Run with environment variables
MEDIA_CONTROL_API_TOKEN="your-secret-token" media-controller

# Or with custom preferred player
MEDIA_CONTROL_PREFERRED_PLAYER="firefox" \
MEDIA_CONTROL_API_TOKEN="your-secret-token" media-controller
```

## Configuration

### Environment Variables

#### Required
- `MEDIA_CONTROL_API_TOKEN`: Bearer token for API authentication (required)

#### Optional  
- `MEDIA_CONTROL_PREFERRED_PLAYER`: Preferred MPRIS player to control (default: "chromium")
  - Examples: "chromium", "firefox", "spotify", "vlc"
  - Case-insensitive substring matching

```bash
# Required
export MEDIA_CONTROL_API_TOKEN="supersecret123"

# Optional - prioritize Firefox instead of Chromium
export MEDIA_CONTROL_PREFERRED_PLAYER="firefox"
```

You can embed these in your systemd unit (see below) or load from an `EnvironmentFile`.

### Systemd Service

#### User Service (\~/.config/systemd/user/media-controller.service)

```ini
[Unit]
Description=Media Controller HTTP Service (user)
After=network.target

[Service]
Type=simple
Environment="MEDIA_CONTROL_API_TOKEN=supersecret123"
Environment="MEDIA_CONTROL_PREFERRED_PLAYER=chromium"
ExecStart=/usr/local/bin/media-controller
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=default.target
```

```bash
# Reload and enable
systemctl --user daemon-reload
systemctl --user enable --now media-controller
```

#### System-wide Service (/etc/systemd/system/media-controller.service)

```ini
[Unit]
Description=Media Controller HTTP Service
After=network.target

[Service]
Type=simple
EnvironmentFile=/etc/media-controller.env
ExecStart=/usr/local/bin/media-controller
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now media-controller
```

## Usage

### Starting the Service

If not using systemd, you can run directly:

```bash
MEDIA_CONTROL_API_TOKEN="supersecret123" ./target/release/media-controller
```

### REST Endpoints

*All endpoints require the header:*

```
Authorization: Bearer <API_TOKEN>
```

| Endpoint         | Method | Description                     |
| :--------------- | :----- | :------------------------------ |
| `/play`          | POST   | Start playback                  |
| `/pause`         | POST   | Pause playback                  |
| `/toggle`        | POST   | Toggle play/pause               |
| `/next`          | POST   | Skip to next track              |
| `/previous`      | POST   | Skip to previous track          |
| `/seek_forward`  | POST   | Seek forward 30 seconds         |
| `/seek_backward` | POST   | Seek backward 30 seconds        |
| `/volume_up`     | POST   | Increase system volume by 5%    |
| `/volume_down`   | POST   | Decrease system volume by 5%    |
| `/status`        | GET    | Get current playback & metadata |

#### Example

```bash
curl -X POST http://192.168.1.111:8080/play \
  -H "Authorization: Bearer supersecret123"
```

## Integration

* **Home Assistant**: Use `rest_command:` or `script:` entries to call these endpoints (see `rest_commands.yaml`).
* **Automations**: Map physical buttons or voice assistants to toggle, skip, volume actions via HTTP.

## Troubleshooting

* **ECONNREFUSED**: Ensure the service is bound to `0.0.0.0` and your firewall allows port 8080.
* **Missing API\_TOKEN**: Verify `Environment=` in systemd or export before starting.
* **Permission Denied**: Check that `pactl` can be run by your user (PulseAudio auth).

## Contributing

Contributions, issues, and feature requests are welcome! Please open an issue or submit a pull request on GitHub.

## License

MIT © \[Your Name]

