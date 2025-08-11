use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Join {
    offer: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JoinResponse {
    answer: String,
}
