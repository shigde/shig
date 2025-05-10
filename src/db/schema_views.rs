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

diesel::table! {
    stream_previews (id) {
        id -> Int4,
        title -> Varchar,
        thumbnail -> Nullable<Varchar>,
        #[max_length = 255]
        uuid -> Varchar,
        description -> Nullable<Text>,
        support -> Nullable<Text>,
        date -> Timestamp,
        start_time -> Nullable<Timestamp>,
        end_time -> Nullable<Timestamp>,
        is_live -> Bool,
        is_public -> Bool,
        owner_name -> Varchar,
        owner_uuid -> Varchar,
        owner_avatar -> Nullable<Varchar>,
        channel_name -> Varchar,
        channel_uuid -> Varchar,
    }
}
