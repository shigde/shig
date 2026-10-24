use diesel::{connection::SimpleConnection, prelude::*, result::Error};
use crate::inbox::{self, Inbox};

#[test]
#[ignore = "Requires SHIG_MIGRATION_TEST_DATABASE_URL pointing to a dedicated PostgreSQL test database"]
fn inbox_events_are_persistent_private_and_idempotent() {
    let url = std::env::var("SHIG_MIGRATION_TEST_DATABASE_URL").unwrap();
    let mut conn = PgConnection::establish(&url).unwrap();
    conn.test_transaction::<_, Error, _>(|conn| {
        conn.batch_execute("CREATE SCHEMA inbox_events_test; SET LOCAL search_path TO inbox_events_test, public;")?;
        crate::db::run_migrations(conn).unwrap();
        conn.batch_execute(
            "INSERT INTO actors (preferred_username, actor_type, actor_iri, created_at) VALUES
                ('host', 'Person', 'http://host', now()), ('guest', 'Person', 'http://guest', now()),
                ('other', 'Person', 'http://other', now()), ('channel', 'Group', 'http://channel', now());
             INSERT INTO users (name,email,user_uuid,user_role_id,password,active,actor_id,created_at) VALUES
                ('host','host@example.test','host',1,'unused',true,1,now()),
                ('guest','guest@example.test','guest',2,'unused',true,2,now()),
                ('other','other@example.test','other',2,'unused',true,3,now());
             INSERT INTO channels (uuid,user_id,actor_id,name,created_at) VALUES ('channel',1,4,'Host channel',now());
             INSERT INTO streams (uuid,user_id,channel_id,title,date,licence,created_at) VALUES ('stream',1,1,'A stream',now(),1,now());
             INSERT INTO lobbies (uuid,user_id,channel_id,stream_id,is_open,created_at) VALUES ('lobby',1,1,1,true,now());
             INSERT INTO stream_friends (user_id,stream_id,friend_role_id,active,accepted,created_at) VALUES
                (2,1,1,true,true,now()),(3,1,1,false,true,now());"
        )?;
        crate::messages::repository::invitation(conn,2,1,1,"host","A stream").unwrap();
        crate::messages::repository::invitation(conn,2,1,1,"host","A stream").unwrap();
        let page = inbox::list(conn,Inbox::Messages,2,0).unwrap();
        assert_eq!((page.total,page.unread),(1,1));
        assert_eq!(page.items[0].action_path.as_deref(),Some("/channel/channel/stream/stream/lobby"));
        assert_eq!(inbox::counts(conn,Inbox::Messages,3).unwrap().total,0);
        assert!(!inbox::mark_read(conn,Inbox::Messages,3,&page.items[0].id).unwrap());
        assert!(inbox::mark_read(conn,Inbox::Messages,2,&page.items[0].id).unwrap());
        assert!(inbox::mark_read(conn,Inbox::Messages,2,&page.items[0].id).unwrap());
        assert_eq!(inbox::counts(conn,Inbox::Messages,2).unwrap().unread,0);
        assert!(inbox::list(conn,Inbox::Messages,2,25).unwrap().items.is_empty());
        for _ in 0..2 { crate::notifications::repository::stream_state(conn,1,true,"host","A stream").unwrap(); }
        assert_eq!(inbox::counts(conn,Inbox::Notifications,2).unwrap().unread,1);
        assert_eq!(inbox::counts(conn,Inbox::Notifications,3).unwrap().total,0);
        assert_eq!(inbox::counts(conn,Inbox::Notifications,1).unwrap().total,0);
        crate::notifications::repository::stream_state(conn,1,false,"host","A stream").unwrap();
        crate::notifications::repository::stream_state(conn,1,true,"host","A stream").unwrap();
        assert_eq!(inbox::counts(conn,Inbox::Notifications,2).unwrap().total,2);
        crate::db::lobbies::update::update_lobby_offline_state(conn,"lobby").unwrap();
        crate::notifications::repository::stream_state(conn,1,true,"host","A stream").unwrap();
        assert_eq!(inbox::counts(conn,Inbox::Notifications,2).unwrap().total,2);
        crate::db::lobbies::update::update_lobby_online_state(conn,"lobby","stream").unwrap();
        crate::notifications::repository::stream_state(conn,1,true,"host","A stream").unwrap();
        assert_eq!(inbox::counts(conn,Inbox::Notifications,2).unwrap().total,3);
        inbox::mark_all_read(conn,Inbox::Messages,3).unwrap();
        assert_eq!(inbox::counts(conn,Inbox::Notifications,2).unwrap().unread,3);
        inbox::mark_all_read(conn,Inbox::Notifications,2).unwrap();
        assert_eq!(inbox::counts(conn,Inbox::Notifications,2).unwrap().unread,0);
        // A deleted stream leaves the durable history but no broken stream action.
        conn.batch_execute("DELETE FROM streams WHERE id=1;")?;
        assert!(inbox::list(conn,Inbox::Messages,2,0).unwrap().items[0].action_path.is_none());
        conn.batch_execute("DELETE FROM users WHERE id=2;")?;
        assert_eq!(inbox::counts(conn,Inbox::Messages,2).unwrap().total,0);
        // Rollback of the migration removes the inbox tables cleanly.
        conn.batch_execute(include_str!("../migrations/2026-10-24-110000_create_inboxes/down.sql"))?;
        conn.batch_execute(include_str!("../migrations/2026-10-24-110000_create_inboxes/up.sql"))?;
        Ok(())
    });
}
