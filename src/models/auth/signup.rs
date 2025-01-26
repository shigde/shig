use crate::db::instances::read::find_home_instance;
use crate::db::instances::Instance;
use crate::db::user_roles::Role;
use crate::db::users::create::create_new_user;
use crate::db::users::User;
use crate::db::verification_tokens::read::find_sing_up_verification_token;
use crate::db::DbPool;
use crate::models::error::ApiError;
use crate::models::mail::config::MailConfig;
use crate::models::mail::Email;
use actix_web::web;
use serde::{Deserialize, Serialize};
use crate::util::domain::split_domain_name;

#[derive(Serialize, Deserialize)]
pub struct SingUp {
    pub name: String,
    pub email: String,
    pub pass: String,
}

impl SingUp {
    pub async fn user(
        pool: &web::Data<DbPool>,
        sing_up: &web::Json<SingUp>,
        cgf: &web::Data<MailConfig>,
    ) -> Result<User, ApiError> {
        let mut conn = pool.get()?;

        let user = create_new_user(
            &mut conn,
            sing_up.name.clone().as_str(),
            sing_up.email.clone().as_str(),
            sing_up.pass.clone().as_str(),
            Role::User,
            false,
        )?;

        if user.active {
            // do nothing if active user already exists
            return Ok(user);
        }

        let token = find_sing_up_verification_token(&mut conn, user.id)?;
        let inst: Instance = find_home_instance(&mut conn)?;
        let link = format!("{}/api/auth/verify/{}", inst.get_base_url(), token.token);

        let (user_name, _) = split_domain_name(user.name.as_str());

        let config = cgf.get_ref().clone();
        let mail = Email::new_activate_account(
            user_name,
            user.email.clone(),
            link,
            inst.domain,
            config,
        );

        mail.send_email().await?;
        Ok(user)
    }
}
