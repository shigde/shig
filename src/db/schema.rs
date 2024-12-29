// @generated automatically by Diesel CLI.

diesel::table! {
    actors (id) {
        id -> Integer,
        preferred_username -> Text,
        actor_type -> Text,
        actor_iri -> Text,
        public_key -> Nullable<Text>,
        private_key -> Nullable<Text>,
        following_iri -> Nullable<Text>,
        followers_iri -> Nullable<Text>,
        inbox_iri -> Nullable<Text>,
        outbox_iri -> Nullable<Text>,
        shared_inbox_iri -> Nullable<Text>,
        instance_id -> Nullable<Integer>,
        remote_created_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    channels (id) {
        id -> Integer,
        user_id -> Integer,
        actor_id -> Integer,
        name -> Text,
        description -> Nullable<Text>,
        support -> Nullable<Text>,
        public -> Bool,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    instances (id) {
        id -> Integer,
        actor_id -> Integer,
        is_home -> Bool,
        domain -> Text,
        tls -> Bool,
        token -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    user_roles (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    users (id) {
        id -> Integer,
        name -> Text,
        email -> Text,
        user_uuid -> Text,
        user_role_id -> Integer,
        password -> Text,
        active -> Bool,
        actor_id -> Integer,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(channels -> actors (actor_id));
diesel::joinable!(channels -> users (user_id));
diesel::joinable!(instances -> actors (actor_id));
diesel::joinable!(users -> actors (actor_id));
diesel::joinable!(users -> user_roles (user_role_id));

diesel::allow_tables_to_appear_in_same_query!(
    actors,
    channels,
    instances,
    user_roles,
    users,
);
