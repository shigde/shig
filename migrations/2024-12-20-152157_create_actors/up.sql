create table actors
(
    id                 INTEGER              NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    preferred_username varchar(255)         NOT NULL,
    actor_type         VARCHAR(40)          NOT NULL,
    actor_iri          varchar(2000) UNIQUE NOT NULL,
    public_key         varchar(5000),
    private_key        varchar(5000),
    following_iri      varchar(2000),
    followers_iri      varchar(2000),
    inbox_iri          varchar(2000),
    outbox_iri         varchar(2000),
    shared_inbox_iri   varchar(2000),
    instance_id        INTEGER,
    remote_created_at  TIMESTAMP,
    created_at         TIMESTAMP            NOT NULL,
    updated_at         TIMESTAMP            NOT NULL DEFAULT now()
);

create unique index actors_iri_index on actors (actor_iri);

-- create unique index actors_preferred_username_lower_instance_id
--     on actors (lower("preferred_username"::text), """instance_id""")
--     where ("instance_id" IS NOT NULL);

create index actors_inbox_url_shared_inbox_url
    on actors ("inbox_iri", "shared_inbox_iri");

create index actors_shared_inbox_url
    on actors ("shared_inbox_iri");

create index actor_instance_id
    on actors ("instance_id");

create index actors_followers_url
    on actors ("followers_iri");
