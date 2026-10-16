# Distributed live-streaming for the Fediverse

- Shig enables distributed live streams via the Ferdivers. 
- Shig scales primarily through your server community, not through the viewers. 
- Shig distributes the streaming costs across the different server providers.

## Get Started

For a local development setup:

```sh
brew install postgresql gstreamer
cargo install diesel_cli --no-default-features --features postgres
createdb shig
echo DATABASE_URL=postgres://postgres@localhost:5432/shig > .env
diesel migration run
cargo run -- --config config/dev.toml
```

Livestream publishing uses an in-process GStreamer pipeline (>= 1.24), not FFmpeg. See [Relay Architecture](doc/architecture/relay.md) for required plugins and Linux packages.

## Install

For production setup, start with [Shig Server](doc/admin/shig-server.md) and [Production](doc/admin/production.md).

## Configuration

The server configuration is documented in [Config](doc/config.md).

## Documentation

Please follow this link: [Doc](doc/index.md).

