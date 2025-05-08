diesel::table! {
    active_users (id) {
        id -> Int4,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        user_uuid -> Varchar,
        user_role_id -> Int4,
        user_actor_id -> Int4,
        channel_id -> Int4,
        #[max_length = 255]
        channel_uuid -> Varchar,
        channel_actor_id -> Int4,
        #[max_length = 255]
        avatar -> Nullable<Varchar>,
    }
}
