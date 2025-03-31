diesel::table! {
    session_principals_view (id) {
        id -> Int4,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        user_uuid -> Varchar,
        user_role_id -> Int4,
        #[max_length = 2000]
        user_actor -> Varchar,
        #[max_length = 2000]
        channel_actor -> Varchar,
    }
}
