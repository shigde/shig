pub mod create;
pub mod read;
pub mod update;

pub fn set(conn: &mut diesel::PgConnection, user_id: i32, settings: &str) -> crate::db::error::DbResult<()> {
    if update::merge(conn, user_id, settings)? == 0 {
        create::insert(conn, user_id, settings)?;
    }
    Ok(())
}
