use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error};

const ORIGINAL: &str =
    include_str!("../../migrations/2025-08-20-184207_create_stream_friends/up.sql");
const UP: &str =
    include_str!("../../migrations/2026-10-16-090000_fix_stream_friends_uniqueness/up.sql");
const DOWN: &str =
    include_str!("../../migrations/2026-10-16-090000_fix_stream_friends_uniqueness/down.sql");

fn insert(conn: &mut PgConnection, stream: i32, user: i32) -> QueryResult<usize> {
    diesel::sql_query(
        "INSERT INTO stream_friends (stream_id, user_id, created_at) VALUES ($1, $2, now())",
    )
    .bind::<diesel::sql_types::Integer, _>(stream)
    .bind::<diesel::sql_types::Integer, _>(user)
    .execute(conn)
}

fn assert_unique_violation(result: QueryResult<usize>) {
    assert!(matches!(
        result,
        Err(Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _))
    ));
}

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[test]
#[ignore = "Requires SHIG_MIGRATION_TEST_DATABASE_URL pointing to a dedicated PostgreSQL test database"]
fn stream_friends_migration_preserves_memberships_and_enforces_pairs() {
    let url = std::env::var("SHIG_MIGRATION_TEST_DATABASE_URL")
        .expect("dedicated test database URL required");
    let mut conn = PgConnection::establish(&url).unwrap();
    conn.test_transaction::<_, Error, _>(|conn| {
        conn.batch_execute(
            "CREATE SCHEMA stream_friends_migration_test;
             SET LOCAL search_path TO stream_friends_migration_test;
             CREATE TABLE users (id INTEGER PRIMARY KEY);
             CREATE TABLE streams (id INTEGER PRIMARY KEY);
             CREATE TABLE friend_roles (id INTEGER PRIMARY KEY);
             INSERT INTO users VALUES (1), (2);
             INSERT INTO streams VALUES (1), (2);
             INSERT INTO friend_roles VALUES (1);",
        )?;
        conn.batch_execute(ORIGINAL)?;
        insert(conn, 1, 1)?;
        conn.batch_execute(UP)?;
        // Rollback is reversible while the old one-to-one restrictions still hold.
        conn.batch_execute(DOWN)?;
        conn.batch_execute(UP)?;
        insert(conn, 1, 2)?;
        insert(conn, 2, 1)?;
        assert_unique_violation(conn.transaction(|conn| insert(conn, 1, 1)));
        let count: Count =
            diesel::sql_query("SELECT count(*) AS count FROM stream_friends").get_result(conn)?;
        assert_eq!(count.count, 3);
        // An incompatible rollback must fail without removing any memberships or indexes.
        let rollback = conn.transaction::<_, Error, _>(|conn| conn.batch_execute(DOWN));
        assert!(matches!(
            rollback,
            Err(Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _))
        ));
        let count: Count =
            diesel::sql_query("SELECT count(*) AS count FROM stream_friends").get_result(conn)?;
        assert_eq!(count.count, 3);
        assert_unique_violation(conn.transaction(|conn| insert(conn, 1, 1)));
        // Different pairs are still accepted after the failed rollback.
        insert(conn, 2, 2)?;
        Ok(())
    });
}

#[test]
#[ignore = "Requires SHIG_MIGRATION_TEST_DATABASE_URL pointing to a dedicated PostgreSQL test database"]
fn channel_friends_migration_preserves_memberships_and_enforces_pairs() {
    let url = std::env::var("SHIG_MIGRATION_TEST_DATABASE_URL")
        .expect("dedicated test database URL required");
    let mut conn = PgConnection::establish(&url).unwrap();
    conn.test_transaction::<_, Error, _>(|conn| {
        conn.batch_execute("CREATE SCHEMA channel_friends_migration_test;
            SET LOCAL search_path TO channel_friends_migration_test;
            CREATE TABLE users (id INTEGER PRIMARY KEY);
            CREATE TABLE channels (id INTEGER PRIMARY KEY);
            CREATE TABLE friend_roles (id INTEGER PRIMARY KEY);
            INSERT INTO users VALUES (1), (2);
            INSERT INTO channels VALUES (1), (2);
            INSERT INTO friend_roles VALUES (1);")?;
        conn.batch_execute(include_str!("../../migrations/2025-08-20-184124_create_channel_friends/up.sql"))?;
        let add = |conn: &mut PgConnection, channel: i32, user: i32| {
            diesel::sql_query("INSERT INTO channel_friends (channel_id, user_id, created_at) VALUES ($1, $2, now())")
                .bind::<diesel::sql_types::Integer, _>(channel).bind::<diesel::sql_types::Integer, _>(user).execute(conn)
        };
        let up = include_str!("../../migrations/2026-10-16-100000_fix_channel_friends_uniqueness/up.sql");
        let down = include_str!("../../migrations/2026-10-16-100000_fix_channel_friends_uniqueness/down.sql");
        add(conn, 1, 1)?;
        conn.batch_execute(up)?;
        conn.batch_execute(down)?;
        conn.batch_execute(up)?;
        add(conn, 1, 2)?;
        add(conn, 2, 1)?;
        assert_unique_violation(conn.transaction(|conn| add(conn, 1, 1)));
        let rollback = conn.transaction::<_, Error, _>(|conn| conn.batch_execute(down));
        assert!(matches!(rollback, Err(Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _))));
        let count: Count = diesel::sql_query("SELECT count(*) AS count FROM channel_friends").get_result(conn)?;
        assert_eq!(count.count, 3);
        add(conn, 2, 2)?;
        Ok(())
    });
}

#[test]
#[ignore = "Requires SHIG_MIGRATION_TEST_DATABASE_URL pointing to a dedicated PostgreSQL test database"]
fn view_joins_timestamps_and_migration_rollbacks() {
    let url = std::env::var("SHIG_MIGRATION_TEST_DATABASE_URL")
        .expect("dedicated test database URL required");
    let mut conn = PgConnection::establish(&url).unwrap();
    conn.test_transaction::<_, Error, _>(|conn| {
        conn.batch_execute("CREATE SCHEMA view_migration_test; SET LOCAL search_path TO view_migration_test;")?;
        crate::db::run_migrations(conn).unwrap();
        conn.batch_execute(
            "INSERT INTO actors (preferred_username, actor_type, actor_iri, created_at) VALUES
                ('alice', 'Person', 'https://example.test/alice', now()),
                ('channel', 'Group', 'https://example.test/channel', now()),
                ('bob', 'Person', 'https://example.test/bob', now());
             INSERT INTO users (name, email, user_uuid, user_role_id, password, active, actor_id, created_at, updated_at) VALUES
                ('alice', 'alice@example.test', 'alice', 2, 'unused', true, 1, now(), '2000-01-01'),
                ('bob', 'bob@example.test', 'bob', 2, 'unused', true, 3, now(), '2000-01-01');
             INSERT INTO channels (uuid, user_id, actor_id, name, created_at) VALUES
                ('channel', 1, 2, 'Channel', now());
             INSERT INTO actor_images (filename, file_url, on_disk, image_type, actor_id, width, created_at) VALUES
                ('banner', '/banner', true, 'banner', 3, 1024, now()),
                ('small', '/small', true, 'actor', 3, 64, now()),
                ('large', '/large', true, 'actor', 3, 256, now()),
                ('new', '/avatar', true, 'actor', 3, 256, now());
             INSERT INTO streams (uuid, user_id, channel_id, title, date, licence, created_at) VALUES
                ('stream', 2, 1, 'Stream', now(), 1, now());
             DO $$ BEGIN
                ASSERT (SELECT count(*) FROM active_users WHERE user_uuid = 'bob') = 1;
                ASSERT (SELECT avatar FROM active_users WHERE user_uuid = 'bob') = '/avatar';
                ASSERT (SELECT count(*) FROM stream_previews WHERE uuid = 'stream') = 1;
                ASSERT (SELECT owner_avatar FROM stream_previews WHERE uuid = 'stream') = '/avatar';
                ASSERT (SELECT channel_uuid FROM stream_previews WHERE uuid = 'stream') = 'channel';
                ASSERT (SELECT avatar IS NULL FROM active_users WHERE user_uuid = 'alice');
             END $$;
             UPDATE users SET name = 'Bob updated' WHERE user_uuid = 'bob';
             DO $$ BEGIN
                ASSERT (SELECT updated_at > '2000-01-01' FROM users WHERE user_uuid = 'bob');
             END $$;
             UPDATE users SET name = 'Bob explicit', updated_at = '2001-01-01' WHERE user_uuid = 'bob';
             DO $$ BEGIN
                ASSERT (SELECT updated_at = '2001-01-01' FROM users WHERE user_uuid = 'bob');
             END $$;"
        )?;
        // Exercise every down migration, including the repaired historical ones.
        use diesel_migrations::MigrationHarness;
        conn.revert_all_migrations(crate::db::MIGRATIONS).unwrap();
        crate::db::run_migrations(conn).unwrap();
        Ok(())
    });
}
