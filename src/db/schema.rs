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
        server_id -> Nullable<Integer>,
        remote_created_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    instances (id) {
        id -> Integer,
        actor_id -> Integer,
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
        uuid -> Text,
        role_id -> Integer,
        password -> Text,
        active -> Bool,
        actor_id -> Integer,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(instances -> actors (actor_id));
diesel::joinable!(users -> actors (actor_id));
diesel::joinable!(users -> user_roles (role_id));

diesel::allow_tables_to_appear_in_same_query!(
    actors,
    instances,
    user_roles,
    users,
);
