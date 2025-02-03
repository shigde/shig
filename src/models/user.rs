use serde::{Deserialize, Serialize};
use crate::models::auth::session::Principal;
use crate::util::domain::split_domain_name;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub name: String,
    pub domain: String,
    pub role: i32,
}

impl User {
    pub fn from_principal(principal: Principal) -> User {
        let (name, domain) = split_domain_name(principal.name.as_str());
        User {
            name,
            domain,
            role: principal.user_role_id,
        }
    }
}
