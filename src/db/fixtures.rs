use crate::db::error::DbResult;
use crate::db::instances::create::upsert_new_instance;
use crate::db::user_roles::Role;
use crate::db::users::create::create_new_user;
use crate::db::users::read::exists_user_by_email;
use crate::federation::FederationConfig;
use diesel::PgConnection;

pub fn insert_fixtures(conn: &mut PgConnection, cfg: FederationConfig) -> DbResult<()> {
    create_home_instance(conn, cfg.clone())?;
    create_standard_users(conn, cfg.clone())?;
    Ok(())
}

pub fn create_home_instance(conn: &mut PgConnection, cfg: FederationConfig) -> DbResult<()> {
    if cfg.enable {
        upsert_new_instance(
            conn,
            cfg.instance.as_str(),
            true,
            cfg.domain.as_str(),
            cfg.tls,
            Some(cfg.token.as_str()),
        )
        .map_err(|e| -> String { format!("upsert new instance: {}", e) })?;
        Ok(())
    } else {
        Ok(())
    }
}

pub fn create_standard_users(conn: &mut PgConnection, cfg: FederationConfig) -> DbResult<()> {
    let domain = cfg.domain.as_str();
    let user_email = format!("user@{domain}");
    let admin_email = format!("admin@{domain}");

    let user_exists = exists_user_by_email(conn, user_email.clone().as_str())?;
    let admin_exists = exists_user_by_email(conn, admin_email.clone().as_str())?;

    if user_exists == false {
        create_new_user(
            conn,
            "user",
            user_email.clone().as_str(),
            "user",
            Role::User,
            true,
        )?;
    }
    if admin_exists == false {
        create_new_user(
            conn,
            "admin",
            admin_email.clone().as_str(),
            "admin",
            Role::Admin,
            true,
        )?;
    }
    Ok(())
}
