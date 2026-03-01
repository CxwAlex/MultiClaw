# Docker Setup Guide

This guide explains how to run MultiClaw in Docker mode, including bootstrap, onboarding, and daily usage.

## Prerequisites

- [Docker](https://docs.docker.com/engine/install/) or [Podman](https://podman.io/getting-started/installation)
- Git

## Quick Start

### 1. Bootstrap in Docker Mode

```bash
# Clone the repository
git clone https://github.com/multiclaw-labs/multiclaw.git
cd multiclaw

# Run bootstrap with Docker mode
./bootstrap.sh --docker
```

This builds the Docker image and prepares the data directory. Onboarding is **not** run by default in Docker mode.

### 2. Run Onboarding

After bootstrap completes, run onboarding inside Docker:

```bash
# Interactive onboarding (recommended for first-time setup)
./multiclaw_install.sh --docker --interactive-onboard

# Or non-interactive with API key
./multiclaw_install.sh --docker --api-key "sk-..." --provider openrouter
```

### 3. Start MultiClaw

#### Daemon Mode (Background Service)

```bash
# Start as a background daemon
./multiclaw_install.sh --docker --docker-daemon

# Check logs
docker logs -f multiclaw-daemon

# Stop the daemon
docker rm -f multiclaw-daemon
```

#### Interactive Mode

```bash
# Run a one-off command inside the container
docker run --rm -it \
  -v ~/.multiclaw-docker/.multiclaw:/home/claw/.multiclaw \
  -v ~/.multiclaw-docker/workspace:/workspace \
  multiclaw-bootstrap:local \
  multiclaw agent -m "Hello, MultiClaw!"

# Start interactive CLI mode
docker run --rm -it \
  -v ~/.multiclaw-docker/.multiclaw:/home/claw/.multiclaw \
  -v ~/.multiclaw-docker/workspace:/workspace \
  multiclaw-bootstrap:local \
  multiclaw agent
```

## Configuration

### Data Directory

By default, Docker mode stores data in:
- `~/.multiclaw-docker/.multiclaw/` - Configuration files
- `~/.multiclaw-docker/workspace/` - Workspace files

Override with environment variable:
```bash
MULTICLAW_DOCKER_DATA_DIR=/custom/path ./bootstrap.sh --docker
```

### Pre-seeding Configuration

If you have an existing `config.toml`, you can seed it during bootstrap:

```bash
./bootstrap.sh --docker --docker-config ./my-config.toml
```

### Using Podman

```bash
MULTICLAW_CONTAINER_CLI=podman ./bootstrap.sh --docker
```

## Common Commands

| Task | Command |
|------|---------|
| Start daemon | `./multiclaw_install.sh --docker --docker-daemon` |
| View daemon logs | `docker logs -f multiclaw-daemon` |
| Stop daemon | `docker rm -f multiclaw-daemon` |
| Run one-off agent | `docker run --rm -it ... multiclaw agent -m "message"` |
| Interactive CLI | `docker run --rm -it ... multiclaw agent` |
| Check status | `docker run --rm -it ... multiclaw status` |
| Start channels | `docker run --rm -it ... multiclaw channel start` |

Replace `...` with the volume mounts shown in [Interactive Mode](#interactive-mode).

## Reset Docker Environment

To completely reset your Docker MultiClaw environment:

```bash
./bootstrap.sh --docker --docker-reset
```

This removes:
- Docker containers
- Docker networks
- Docker volumes
- Data directory (`~/.multiclaw-docker/`)

## Troubleshooting

### "multiclaw: command not found"

This error occurs when trying to run `multiclaw` directly on the host. In Docker mode, you must run commands inside the container:

```bash
# Wrong (on host)
multiclaw agent

# Correct (inside container)
docker run --rm -it \
  -v ~/.multiclaw-docker/.multiclaw:/home/claw/.multiclaw \
  -v ~/.multiclaw-docker/workspace:/workspace \
  multiclaw-bootstrap:local \
  multiclaw agent
```

### No Containers Running After Bootstrap

Running `./bootstrap.sh --docker` only builds the image and prepares the data directory. It does **not** start a container. To start MultiClaw:

1. Run onboarding: `./multiclaw_install.sh --docker --interactive-onboard`
2. Start daemon: `./multiclaw_install.sh --docker --docker-daemon`

### Container Fails to Start

Check Docker logs for errors:
```bash
docker logs multiclaw-daemon
```

Common issues:
- Missing API key: Run onboarding with `--api-key` or edit `config.toml`
- Permission issues: Ensure Docker has access to the data directory

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MULTICLAW_DOCKER_DATA_DIR` | Data directory path | `~/.multiclaw-docker` |
| `MULTICLAW_DOCKER_IMAGE` | Docker image name | `multiclaw-bootstrap:local` |
| `MULTICLAW_CONTAINER_CLI` | Container CLI (docker/podman) | `docker` |
| `MULTICLAW_DOCKER_DAEMON_NAME` | Daemon container name | `multiclaw-daemon` |
| `MULTICLAW_DOCKER_CARGO_FEATURES` | Build features | (empty) |

## Related Documentation

- [Quick Start](../README.md#quick-start)
- [Configuration Reference](config-reference.md)
- [Operations Runbook](operations-runbook.md)
