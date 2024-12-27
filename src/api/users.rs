use actix_web::{Responder};
use serde_derive::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct InputUser {
    pub name: String,
    pub email: String,
    pub password: String,
}

// pub async fn get_users(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
//     Ok(web::block(move || get_all_users(pool))
//         .await
//         .map(|user| HttpResponse::Ok().json(user))
//         .map_err(|_| HttpResponse::InternalServerError())?
//     )
// }

// fn get_all_users(pool: web::Data<DbPool>) -> Result<Vec<User>, diesel::result::Error> {
//     let conn = pool.get();
//
//     let conn = web::block(move || pool.get())
//         .await?
//         .map_err(error::ErrorInternalServerError)?;
//
//     let items = users.load::<User>(conn)?;
//     //Ok(items)
// }

#[allow(dead_code)]
pub async fn get_user_by_id() -> impl Responder {
    format!("hello from get users by id")
}

#[allow(dead_code)]
pub async fn add_user() -> impl Responder {
    format!("hello from add user")
}

#[allow(dead_code)]
pub async fn delete_user() -> impl Responder {
    format!("hello from delete user")
}
