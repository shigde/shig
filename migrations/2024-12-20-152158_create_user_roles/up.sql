CREATE TABLE user_roles
(
    id   INTEGER NOT NULL PRIMARY KEY,
    name VARCHAR NOT NULL
);

INSERT INTO "user_roles" ("id", "name")
VALUES (1, 'admin');
INSERT INTO "user_roles" ("id", "name")
VALUES (2, 'user');
INSERT INTO "user_roles" ("id", "name")
VALUES (3, 'guest');
INSERT INTO "user_roles" ("id", "name")
VALUES (4, 'application');
INSERT INTO "user_roles" ("id", "name")
VALUES (5, 'service');

