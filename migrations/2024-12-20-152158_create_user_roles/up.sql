CREATE TABLE user_roles
(
    id         INTEGER        NOT NULL PRIMARY KEY AUTOINCREMENT,
    name       TEXT           NOT NULL
);

INSERT INTO "user_roles" ("name") VALUES ('admin');
INSERT INTO "user_roles" ("name") VALUES ('user');
INSERT INTO "user_roles" ("name") VALUES ('guest');
INSERT INTO "user_roles" ("name") VALUES ('application');
INSERT INTO "user_roles" ("name") VALUES ('service');

