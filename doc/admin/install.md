# Install

## Get started (Debian/ Ubuntu)

### 1. Download the latest release using curl

Use `curl` to download the Shig directly from the release page. Replace ` APP_REL=<release>` with the actual Release of the artifact:

```bash
  export APP_REL=0.1.0
  curl -L -o shig_server.tar.gz https://github.com/shigde/shig/releases/download/${APP_REL}/shig_server-${APP_REL}-x86_64-unknown-linux-gnu.tar.gz
```

### 2. Extract artefact

Extract the downloaded artifact using the command:

```bash
  tar -xvzf shig_server.tar.gz -C ./
 ```

### 4. Install and run

Switch to the extracted folder, adjust the TOML configuration, and start the server.

```bash
  cd shig_server-...
  shig_server -c default.toml
```
