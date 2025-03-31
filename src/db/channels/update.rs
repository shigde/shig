use crate::db::error::DbResult;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, AsChangeset, Clone)]
#[diesel(table_name = crate::db::schema::channels)]
pub struct ChannelUpdate {
    pub name: String,
    pub description: Option<String>,
    pub support: Option<String>,
    pub public: bool,
}

pub fn update(conn: &mut PgConnection, channel_id: i32, channel: ChannelUpdate) -> DbResult<()> {
    use crate::db::schema::channels::dsl::channels;
    use crate::db::schema::channels::dsl::id;
    diesel::update(channels.filter(id.eq(channel_id)))
        .set::<ChannelUpdate>(channel)
        .execute(conn)?;
    Ok(())
}
