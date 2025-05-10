// @generated automatically by Diesel CLI.

diesel::table! {
    actor_images (id) {
        id -> Int4,
        #[max_length = 255]
        filename -> Varchar,
        height -> Nullable<Int4>,
        width -> Nullable<Int4>,
        #[max_length = 255]
        file_url -> Nullable<Varchar>,
        on_disk -> Bool,
        #[max_length = 40]
        image_type -> Varchar,
        actor_id -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    actors (id) {
        id -> Int4,
        #[max_length = 255]
        preferred_username -> Varchar,
        #[max_length = 40]
        actor_type -> Varchar,
        #[max_length = 2000]
        actor_iri -> Varchar,
        #[max_length = 5000]
        public_key -> Nullable<Varchar>,
        #[max_length = 5000]
        private_key -> Nullable<Varchar>,
        #[max_length = 2000]
        following_iri -> Nullable<Varchar>,
        #[max_length = 2000]
        followers_iri -> Nullable<Varchar>,
        #[max_length = 2000]
        inbox_iri -> Nullable<Varchar>,
        #[max_length = 2000]
        outbox_iri -> Nullable<Varchar>,
        #[max_length = 2000]
        shared_inbox_iri -> Nullable<Varchar>,
        instance_id -> Nullable<Int4>,
        remote_created_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    channels (id) {
        id -> Int4,
        #[max_length = 255]
        uuid -> Varchar,
        user_id -> Int4,
        actor_id -> Int4,
        name -> Varchar,
        description -> Nullable<Text>,
        support -> Nullable<Text>,
        public -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    instances (id) {
        id -> Int4,
        actor_id -> Int4,
        is_home -> Bool,
        #[max_length = 255]
        domain -> Varchar,
        tls -> Bool,
        #[max_length = 500]
        token -> Nullable<Varchar>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    stream_meta_data (id) {
        id -> Int4,
        stream_id -> Int4,
        is_shig -> Bool,
        #[max_length = 255]
        stream_key -> Varchar,
        #[max_length = 255]
        url -> Varchar,
        protocol -> Int4,
        permanent_live -> Bool,
        save_replay -> Bool,
        latency_mode -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    stream_participants (id) {
        id -> Int4,
        stream_id -> Int4,
        user_id -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    stream_thumbnails (id) {
        id -> Int4,
        #[max_length = 255]
        filename -> Varchar,
        height -> Nullable<Int4>,
        width -> Nullable<Int4>,
        #[max_length = 255]
        file_url -> Nullable<Varchar>,
        on_disk -> Bool,
        stream_id -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    streams (id) {
        id -> Int4,
        #[max_length = 255]
        uuid -> Varchar,
        user_id -> Int4,
        channel_id -> Int4,
        title -> Varchar,
        description -> Nullable<Text>,
        support -> Nullable<Text>,
        date -> Timestamp,
        start_time -> Nullable<Timestamp>,
        end_time -> Nullable<Timestamp>,
        licence -> Int4,
        is_repeating -> Bool,
        repeat -> Nullable<Int4>,
        is_public -> Bool,
        is_live -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    user_roles (id) {
        id -> Int4,
        #[max_length = 40]
        name -> Varchar,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        user_uuid -> Varchar,
        user_role_id -> Int4,
        #[max_length = 255]
        password -> Varchar,
        active -> Bool,
        actor_id -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    verification_tokens (id) {
        id -> Int4,
        user_id -> Int4,
        #[max_length = 255]
        kind -> Varchar,
        #[max_length = 255]
        token -> Varchar,
        verified -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(actor_images -> actors (actor_id));
diesel::joinable!(channels -> actors (actor_id));
diesel::joinable!(channels -> users (user_id));
diesel::joinable!(instances -> actors (actor_id));
diesel::joinable!(stream_meta_data -> streams (stream_id));
diesel::joinable!(stream_participants -> streams (stream_id));
diesel::joinable!(stream_participants -> users (user_id));
diesel::joinable!(stream_thumbnails -> streams (stream_id));
diesel::joinable!(streams -> channels (channel_id));
diesel::joinable!(streams -> users (user_id));
diesel::joinable!(users -> actors (actor_id));
diesel::joinable!(users -> user_roles (user_role_id));
diesel::joinable!(verification_tokens -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    actor_images,
    actors,
    channels,
    instances,
    stream_meta_data,
    stream_participants,
    stream_thumbnails,
    streams,
    user_roles,
    users,
    verification_tokens,
);
