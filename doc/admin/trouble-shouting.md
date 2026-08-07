# trouble-shouting

### `GLIBC_2.39 not found`

The release binary was built on a too-new Ubuntu runner. Build on `ubuntu-22.04`:

```yaml
runs-on: ubuntu-22.04
```

Create a new release asset after changing the workflow.

### `cannot find -lpq`

Install `libpq-dev` during the GitHub build:

```shell
sudo apt-get install -y libpq-dev pkg-config
```

Do not use `pq-sys` bundled unless you have tested the C build for your target.

### `failed to open certificate file`

Check Shig can read the copied cert files:

```shell
sudo -u shig test -r /opt/shig/certs/relay/fullchain.pem && echo cert-ok
sudo -u shig test -r /opt/shig/certs/relay/privkey.pem && echo key-ok
```

See [config](config.md#relay) for the expected relay TLS paths.

### `Permission denied (os error 13)` during startup

Check writable runtime directories:

```shell
sudo -u shig test -w /opt/shig && echo opt-shig-writable
```

If the app should create `htdocs`, `/opt/shig` must be writable by `shig`.

### `invalid percent-encoded token`

The database URL contains an unescaped `%` or another special character. URL-encode the password.

See [config](config.md#database) for the database URL format.

### Signup takes 60 seconds and returns 500

SMTP is probably unreachable or misconfigured. Set:

```toml
[mail]
enable = false
```

or fix SMTP and test it with `openssl`, `nc`, or `swaks`.

See [config](config.md#mail) for the mail options.

### `gh release download` says `not a git repository`

Use:

```shell
gh release download "$RELEASE_TAG" --repo "$GITHUB_REPOSITORY"
```

The deploy workflow does not need a checkout if the repo is passed explicitly.
