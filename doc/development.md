# Develop on Shig


## Setup Postgres DB

#### Mac
```sh
brew install postgres
```
### Linux
```sh
..
```



### Recreate DB Schema

```sh
cargo install diesel_cli --no-default-features --features postgres
```

```sh
echo DATABASE_URL=postgres://postgres@localhost:5432/shig > .env
```

Create Database

```sql
create database shig
    with owner postgres;
```

## Init Tables

```sh
diesel migration run
```

## Create Database Shema
```sh
diesel print-schema
```


