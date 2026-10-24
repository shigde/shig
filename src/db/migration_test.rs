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

const VERIFY_DEFAULTS_UP: &str = include_str!("../../migrations/2026-10-24-100000_verify_default_accounts/up.sql");
const VERIFY_DEFAULTS_DOWN: &str = include_str!("../../migrations/2026-10-24-100000_verify_default_accounts/down.sql");

#[test]
#[ignore = "Requires SHIG_MIGRATION_TEST_DATABASE_URL pointing to a dedicated PostgreSQL test database"]
fn default_account_verification_migration_is_scoped_and_reversible() {
    let url = std::env::var("SHIG_MIGRATION_TEST_DATABASE_URL").unwrap();
    let mut conn = PgConnection::establish(&url).unwrap();
    conn.test_transaction::<_, Error, _>(|conn| {
        conn.batch_execute("CREATE SCHEMA default_verification_test; SET LOCAL search_path TO default_verification_test, public;")?;
        crate::db::run_migrations(conn).unwrap();
        conn.batch_execute(
            "INSERT INTO actors (preferred_username, actor_type, actor_iri, created_at) VALUES ('instance', 'Application', 'http://localhost', now());
             INSERT INTO instances (actor_id, is_home, domain, tls, created_at) VALUES (1, true, 'localhost:4200', false, now());
             INSERT INTO actors (preferred_username, actor_type, actor_iri, instance_id, created_at) VALUES
                ('admin', 'Person', 'http://localhost/admin', 1, now()),
                ('user', 'Person', 'http://localhost/user', 1, now()),
                ('alice', 'Person', 'http://localhost/alice', 1, now()),
                ('admin', 'Person', 'http://remote/admin', NULL, now());
             INSERT INTO users (name, email, user_uuid, user_role_id, password, active, actor_id, created_at) VALUES
                ('admin@localhost:4200', 'admin@localhost.de', 'default-admin', 1, 'unchanged', false, 2, now()),
                ('user@localhost:4200', 'user@localhost.de', 'default-user', 2, 'unchanged', true, 3, now()),
                ('alice@localhost:4200', 'alice@example.test', 'alice', 2, 'unchanged', true, 4, now()),
                ('admin@remote.test', 'admin@remote.test', 'remote-admin', 1, 'unchanged', true, 5, now());
             INSERT INTO verification_tokens (user_id, kind, token, verified, created_at) VALUES
                (3, 'sign_up', 'actual-email-verification', true, now());"
        )?;
        conn.batch_execute(VERIFY_DEFAULTS_UP)?;
        conn.batch_execute(VERIFY_DEFAULTS_UP)?;
        conn.batch_execute(
            "DO $$ BEGIN
               ASSERT (SELECT count(*) FROM verification_tokens WHERE token LIKE 'default-account-verified:%') = 2;
               ASSERT (SELECT bool_and(verified) FROM verification_tokens WHERE user_id IN (1, 2));
               ASSERT NOT (SELECT active FROM users WHERE user_uuid = 'default-admin');
               ASSERT (SELECT active FROM users WHERE user_uuid = 'default-user');
               ASSERT (SELECT bool_and(password = 'unchanged') FROM users);
               ASSERT NOT EXISTS (SELECT 1 FROM verification_tokens WHERE user_id = 4);
             END $$;"
        )?;
        conn.batch_execute(VERIFY_DEFAULTS_DOWN)?;
        conn.batch_execute("DO $$ BEGIN ASSERT (SELECT count(*) FROM verification_tokens) = 1; ASSERT (SELECT verified FROM verification_tokens WHERE token = 'actual-email-verification'); END $$;")?;
        Ok(())
    });
}

#[test]
#[ignore = "Requires SHIG_MIGRATION_TEST_DATABASE_URL pointing to a dedicated PostgreSQL test database"]
fn newly_provisioned_defaults_are_verified_without_changing_regular_signup() {
    let url = std::env::var("SHIG_MIGRATION_TEST_DATABASE_URL").unwrap();
    let mut conn = PgConnection::establish(&url).unwrap();
    conn.test_transaction::<_, Error, _>(|conn| {
        conn.batch_execute("CREATE SCHEMA default_provisioning_test; SET LOCAL search_path TO default_provisioning_test, public;")?;
        crate::db::run_migrations(conn).unwrap();
        conn.batch_execute(
            "INSERT INTO actors (preferred_username, actor_type, actor_iri, created_at) VALUES ('instance', 'Application', 'http://localhost', now());
             INSERT INTO instances (actor_id, is_home, domain, tls, created_at) VALUES (1, true, 'localhost:4200', false, now());"
        )?;
        let cfg = crate::federation::FederationConfig { enable: true, domain: "localhost:4200".into(), instance: "test".into(), token: "unused".into(), tls: false };
        crate::db::fixtures::create_standard_users(conn, cfg.clone()).unwrap();
        crate::db::fixtures::create_standard_users(conn, cfg.clone()).unwrap();
        let page = crate::db::users::admin::list(conn, "", 0).unwrap();
        assert_eq!(page.total, 2);
        assert!(page.users.iter().all(|user| user.active && user.verified));
        let admin = page.users.iter().find(|user| user.name.starts_with("admin@")).unwrap();
        let actor = crate::db::users::User::from_uuid(conn, admin.uuid.clone()).unwrap();
        let target = page.users.iter().find(|user| user.name.starts_with("user@")).unwrap();
        use crate::db::users::admin::{set_role, RoleChange};
        assert_eq!(set_role(conn, actor.id, &target.uuid, 1).unwrap(), RoleChange::Updated);
        assert_eq!(crate::db::active_users::read::find_active_user_by_uuid(conn, &target.uuid).unwrap().user_role_id, 1);
        assert_eq!(set_role(conn, actor.id, &target.uuid, 2).unwrap(), RoleChange::Updated);
        assert_eq!(crate::db::active_users::read::find_active_user_by_uuid(conn, &target.uuid).unwrap().user_role_id, 2);
        let former_admin = crate::db::users::User::from_uuid(conn, target.uuid.clone()).unwrap();
        assert_eq!(set_role(conn, former_admin.id, &admin.uuid, 2).unwrap(), RoleChange::Forbidden);
        assert_eq!(set_role(conn, actor.id, &actor.user_uuid, 2).unwrap(), RoleChange::Forbidden);
        assert_eq!(set_role(conn, actor.id, "missing", 1).unwrap(), RoleChange::NotFound);
        assert!(crate::db::users::admin::set_active(conn, &admin.uuid, false).unwrap());
        crate::db::fixtures::create_standard_users(conn, cfg).unwrap();
        let page = crate::db::users::admin::list(conn, "admin@", 0).unwrap();
        assert!(!page.users[0].active);
        assert!(page.users[0].verified);
        crate::db::users::create::create_new_user(conn, "alice", "alice@example.test", "test", crate::db::user_roles::Role::User, false).unwrap();
        let page = crate::db::users::admin::list(conn, "alice", 0).unwrap();
        assert_eq!(page.total, 1);
        assert!(!page.users[0].active);
        assert!(!page.users[0].verified);
        conn.batch_execute("DO $$ BEGIN ASSERT (SELECT count(*) FROM verification_tokens WHERE kind = 'sign_up' AND verified) = 2; END $$;")?;
        Ok(())
    });
}
