pub mod message;

use crate::db::lobbies::update::{update_lobby_offline_state, update_lobby_online_state};
use crate::db::stream_participants::create::insert_new_stream_participant;
use crate::db::stream_participants::delete::delete_stream_participant_by_user_and_stream_uuid;
use crate::db::DbPool;
use crate::lobby_error;
use crate::sfu::db::message::{AddParticipant, RemoveParticipant, SetLobbyOffline, SetLobbyOnline};
use actix::prelude::*;
use diesel::result::Error;

pub struct DbActor {
    pool: DbPool,
}

impl DbActor {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn set_lobby_online_state(&mut self, lobby_uuid: &str, stream_uuid: &str) -> Result<(), Error> {
        match self.pool.get() {
            Ok(mut conn) => {
                if let Err(e) = update_lobby_online_state(&mut conn, lobby_uuid, stream_uuid) {
                    lobby_error!(
                        lobby_uuid,
                        "Failed to set lobby online. stream_uuid={}, {:?}",
                        stream_uuid,
                        e
                    );
                }
            }
            Err(e) => {
                lobby_error!(
                    lobby_uuid,
                    "Failed to get DB connection to set online state {:?}",
                    e
                );
            }
        };

        Ok(())
    }

    fn set_lobby_offline_state(
        &mut self,
        lobby_uuid: &str,
    ) -> Result<(), Error> {
        match self.pool.get() {
            Ok(mut conn) => {
                if let Err(e) = update_lobby_offline_state(&mut conn, lobby_uuid) {
                    lobby_error!(lobby_uuid, "Failed to set lobby offline {:?}", e);
                }
            }
            Err(e) => {
                lobby_error!(
                    lobby_uuid,
                    "Failed to get DB connection to set offline state {:?}",
                    e
                );
            }
        };

        Ok(())
    }

    fn add_participant(
        &mut self,
        lobby_uuid: &str,
        stream_uuid: &str,
        user_uuid: &str,
    ) -> Result<(), Error> {
        match self.pool.get() {
            Ok(mut conn) => {
                if let Err(e) = insert_new_stream_participant(&mut conn, stream_uuid, user_uuid) {
                    lobby_error!(
                        lobby_uuid,
                        "Failed add participant stream_uuid={}, user_uuid={}, {:?}",
                        stream_uuid,
                        user_uuid,
                        e
                    );
                }
            }
            Err(e) => {
                lobby_error!(
                    lobby_uuid,
                    "Failed to get DB connection to add participant stream_uuid={}, user_uuid={}, {:?}",
                    stream_uuid,
                    user_uuid,
                    e
                );
            }
        };

        Ok(())
    }

    fn remove_participant(
        &mut self,
        lobby_uuid: &str,
        stream_uuid: &str,
        user_uuid: &str,
    ) -> Result<(), Error> {
        match self.pool.get() {
            Ok(mut conn) => {
                if let Err(e) = delete_stream_participant_by_user_and_stream_uuid(
                    &mut conn,
                    stream_uuid,
                    user_uuid,
                ) {
                    lobby_error!(
                        lobby_uuid,
                        "Failed remove participant stream_uuid={}, user_uuid={}, {:?}",
                        stream_uuid,
                        user_uuid,
                        e
                    );
                }
            }
            Err(e) => {
                lobby_error!(
                    lobby_uuid,
                    "Failed to get DB connection to remove participant stream_uuid={}, user_uuid={}, {:?}",
                    stream_uuid,
                    user_uuid,
                    e
                );
            }
        };

        Ok(())
    }
}

impl Actor for DbActor {
    type Context = SyncContext<Self>;
}

impl Handler<SetLobbyOnline> for DbActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: SetLobbyOnline, _: &mut SyncContext<Self>) -> Self::Result {
        log::info!("set lobby online lobby_uuid={}, stream_uuid={}", msg.lobby_uuid.as_str(), msg.stream_uuid.as_str());
        self.set_lobby_online_state(&msg.lobby_uuid.as_str(), &msg.stream_uuid.as_str())
    }
}

impl Handler<SetLobbyOffline> for DbActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: SetLobbyOffline, _: &mut SyncContext<Self>) -> Self::Result {
        log::info!("set lobby offline lobby_uuid={}", msg.lobby_uuid.as_str());
        self.set_lobby_offline_state(&msg.lobby_uuid.as_str())
    }
}

impl Handler<AddParticipant> for DbActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: AddParticipant, _: &mut SyncContext<Self>) -> Self::Result {
        log::info!(
            "add participant lobby_uuid={}, stream_uuid={}, user_uuid={}",
            msg.lobby_uuid,
            msg.stream_uuid,
            msg.user_uuid
        );
        self.add_participant(
            &msg.lobby_uuid.as_str(),
            &msg.stream_uuid.as_str(),
            &msg.user_uuid.as_str(),
        )
    }
}

impl Handler<RemoveParticipant> for DbActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: RemoveParticipant, _: &mut SyncContext<Self>) -> Self::Result {
        log::info!(
            "remove participant lobby_uuid={}, stream_uuid={}, user_uuid={}",
            msg.lobby_uuid,
            msg.stream_uuid,
            msg.user_uuid
        );
        self.remove_participant(
            &msg.lobby_uuid.as_str(),
            &msg.stream_uuid.as_str(),
            &msg.user_uuid.as_str(),
        )
    }
}
