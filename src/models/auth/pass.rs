use crate::db::instances::read::find_home_instance;
use crate::db::instances::Instance;
use crate::db::users::read::find_user_by_email;
use crate::db::users::update::{update_password_by_id, update_password_by_token};
use crate::db::verification_tokens::create::insert_new_verification_token;

use crate::db::verification_tokens::FORGOTTEN_PASSWORD_VERIFICATION_TOKEN;
use crate::db::DbPool;
use crate::models::auth::session::Principal;
use crate::models::error::ApiError;
use crate::models::mail::config::MailConfig;
use crate::models::mail::Email;
use crate::util::domain::split_domain_name;
use actix_web::web;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ForgottenPassword {
    pub email: String,
}

impl ForgottenPassword {
    pub async fn send_forgotten_password_email(
        &self,
        pool: &web::Data<DbPool>,
        cgf: &web::Data<MailConfig>,
    ) -> Result<bool, ApiError> {
        let mut conn = pool.get()?;

        let user = find_user_by_email(&mut conn, self.email.clone())?;

        if !user.active {
            return Ok(false);
        }

        let token = insert_new_verification_token(
            &mut conn,
            user.id,
            FORGOTTEN_PASSWORD_VERIFICATION_TOKEN,
        )?;

        let inst: Instance = find_home_instance(&mut conn)?;
        let link = format!("{}/forgotPassword/{}", inst.get_base_url(), token.token);

        let (user_name, _) = split_domain_name(user.name.as_str());

        let config = cgf.get_ref().clone();
        let mail =
            Email::new_forgotten_password(user_name, user.email.clone(), link, inst.domain, config);

        mail.send_email().await?;
        Ok(true)
    }
}

#[derive(Serialize, Deserialize)]
pub struct ResetPassword {
    password: String,
    token: String,
}

impl ResetPassword {
    pub fn set_new_passwort(&self, pool: &web::Data<DbPool>) -> Result<bool, ApiError> {
        let mut conn = pool.get()?;
        let finished = update_password_by_token(&mut conn, &self.token, &self.password).is_ok();
        Ok(finished)
    }
}

#[derive(Serialize, Deserialize)]
pub struct UpdatePassword {
    old_password: String,
    new_password: String,
}

impl UpdatePassword {
    pub fn set_new_passwort(
        &self,
        pool: &web::Data<DbPool>,
        principal: Principal,
    ) -> Result<bool, ApiError> {
        let mut conn = pool.get()?;
        let finished = update_password_by_id(
            &mut conn,
            principal.id,
            &self.new_password,
            &self.old_password,
        )
        .is_ok();
        Ok(finished)
    }
}
