# postgress

Install PostgreSQL:

```shell
sudo apt update
sudo apt install -y postgresql postgresql-client
```

Create the database and user:

```shell
sudo -u postgres psql
```

Example SQL:

```sql
CREATE DATABASE shig;
CREATE USER shig WITH PASSWORD 'change-this-password';
GRANT ALL PRIVILEGES ON DATABASE shig TO shig;
\q
```

Make sure PostgreSQL listens locally:

```shell
sudo ss -ltnp | grep 5432
```

Expected:

```text
127.0.0.1:5432
[::1]:5432
```

If PostgreSQL does not listen on TCP localhost, check:

```shell
sudo nano /etc/postgresql/*/main/postgresql.conf
```

Expected setting:

```conf
listen_addresses = 'localhost'
```

Restart PostgreSQL:

```shell
sudo systemctl restart postgresql
```

Test the connection:

```shell
psql "postgres://shig:change-this-password@127.0.0.1:5432/shig"
```

If the password contains special URL characters, encode it in the database URL.

See [config](config.md#database) for the matching `/opt/shig/config.toml` database settings.

Examples:

```text
%  -> %25
@  -> %40
:  -> %3A
/  -> %2F
#  -> %23
?  -> %3F
&  -> %26
+  -> %2B
```

Shig runs Diesel migrations during startup.
