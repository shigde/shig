use crate::db::error::DbResult;
use crate::db::schema::{channels, users, verification_tokens};
use crate::db::verification_tokens::SIGN_UP_VERIFICATION_TOKEN;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

#[derive(Queryable, Serialize)]
pub struct AdminUser {
    pub uuid: String,
    pub name: String,
    pub email: String,
    pub active: bool,
    pub role: i32,
    pub verified: bool,
    #[serde(serialize_with = "serialize_utc")]
    pub created_at: chrono::NaiveDateTime,
    pub channel_name: Option<String>,
    pub channel_uuid: Option<String>,
}

fn serialize_utc<S: serde::Serializer>(
    date: &chrono::NaiveDateTime,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    DateTime::<Utc>::from_naive_utc_and_offset(*date, Utc).serialize(serializer)
}

#[derive(Serialize)]
pub struct UserPage {
    pub users: Vec<AdminUser>,
    pub total: i64,
}

pub fn list(conn: &mut PgConnection, search: &str, offset: i64) -> DbResult<UserPage> {
    let pattern = format!(
        "%{}%",
        search
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    let query = || {
        users::table.left_join(channels::table).filter(
            diesel::dsl::sql::<diesel::sql_types::Text>("users.name::text")
                .ilike(pattern.clone())
                .or(users::email.ilike(pattern.clone()))
                .or(channels::name.ilike(pattern.clone())),
        )
    };
    let total = query().count().get_result(conn)?;
    let users = query()
        .select((
            users::user_uuid,
            users::name,
            users::email,
            users::active,
            users::user_role_id,
            diesel::dsl::exists(
                verification_tokens::table
                    .filter(verification_tokens::user_id.eq(users::id))
                    .filter(verification_tokens::kind.eq(SIGN_UP_VERIFICATION_TOKEN))
                    .filter(verification_tokens::verified.eq(true)),
            ),
            users::created_at,
            channels::name.nullable(),
            channels::uuid.nullable(),
        ))
        .order((users::created_at.desc(), users::id.desc()))
        .limit(50)
        .offset(offset)
        .load::<AdminUser>(conn)?;
    Ok(UserPage { users, total })
}

pub fn set_active(conn: &mut PgConnection, uuid: &str, enabled: bool) -> DbResult<bool> {
    conn.transaction(|conn| {
        let target = users::table
            .filter(users::user_uuid.eq(uuid))
            .select(users::id)
            .for_update()
            .first::<i32>(conn)
            .optional()?;
        let Some(id) = target else {
            return Ok(false);
        };
        diesel::update(users::table.find(id))
            .set((
                users::active.eq(enabled),
                users::updated_at.eq(Utc::now().naive_utc()),
            ))
            .execute(conn)?;
        if !enabled {
            // Revoke unused activation links; retain completed email verification as evidence.
            diesel::delete(
                verification_tokens::table
                    .filter(verification_tokens::user_id.eq(id))
                    .filter(verification_tokens::kind.eq(SIGN_UP_VERIFICATION_TOKEN))
                    .filter(verification_tokens::verified.eq(false)),
            )
            .execute(conn)?;
        }
        Ok(true)
    })
}

#[derive(Debug, PartialEq)]
pub enum RoleChange {
    Updated,
    NotFound,
    Forbidden,
    UnsupportedRole,
}

pub fn set_role(
    conn: &mut PgConnection,
    actor_id: i32,
    uuid: &str,
    role: i32,
) -> DbResult<RoleChange> {
    use crate::db::user_roles::Role;
    conn.transaction(|conn| {
        // Lock in a stable order and recheck the caller after waiting. Two admins
        // cannot concurrently demote each other and leave no administrator.
        let accounts = users::table
            .filter(users::id.eq(actor_id).or(users::user_uuid.eq(uuid)))
            .order(users::id.asc())
            .for_update()
            .select((
                users::id,
                users::user_uuid,
                users::user_role_id,
                users::active,
            ))
            .load::<(i32, String, i32, bool)>(conn)?;
        if !accounts.iter().any(|(id, _, current_role, active)| {
            *id == actor_id && *current_role == Role::Admin.val() && *active
        }) {
            return Ok(RoleChange::Forbidden);
        }
        let Some((id, _, current_role, _)) = accounts
            .iter()
            .find(|(_, user_uuid, _, _)| user_uuid == uuid)
        else {
            return Ok(RoleChange::NotFound);
        };
        if *id == actor_id || ![Role::Admin.val(), Role::User.val()].contains(&role) {
            return Ok(RoleChange::Forbidden);
        }
        if ![Role::Admin.val(), Role::User.val()].contains(current_role) {
            return Ok(RoleChange::UnsupportedRole);
        }
        diesel::update(users::table.find(*id))
            .set((
                users::user_role_id.eq(role),
                users::updated_at.eq(Utc::now().naive_utc()),
            ))
            .execute(conn)?;
        Ok(RoleChange::Updated)
    })
}
