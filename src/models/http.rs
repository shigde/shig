pub mod response;
pub mod payload;

// Messages
#[allow(dead_code)]
pub const MESSAGE_OK: &str = "ok";
#[allow(dead_code)]
pub const MESSAGE_CAN_NOT_FETCH_DATA: &str = "Can not fetch data";
#[allow(dead_code)]
pub const MESSAGE_CAN_NOT_INSERT_DATA: &str = "Can not insert data";
#[allow(dead_code)]
pub const MESSAGE_CAN_NOT_UPDATE_DATA: &str = "Can not update data";
#[allow(dead_code)]
pub const MESSAGE_CAN_NOT_DELETE_DATA: &str = "Can not delete data";
#[allow(dead_code)]
pub const MESSAGE_SIGNUP_SUCCESS: &str = "Signup successfully";
#[allow(dead_code)]
pub const MESSAGE_SIGNUP_FAILED: &str = "Error while signing up, please try again";
pub const MESSAGE_LOGIN_SUCCESS: &str = "Login successfully";
#[allow(dead_code)]
pub const MESSAGE_LOGIN_FAILED: &str = "Wrong user mail or password, please try again";
#[allow(dead_code)]
pub const MESSAGE_USER_NOT_FOUND: &str = "User not found, please signup";
#[allow(dead_code)]
pub const MESSAGE_LOGOUT_SUCCESS: &str = "Logout successfully";
#[allow(dead_code)]
pub const MESSAGE_PROCESS_TOKEN_ERROR: &str = "Error while processing token";
pub const MESSAGE_INVALID_TOKEN: &str = "Invalid token, please login again";
pub const MESSAGE_INTERNAL_SERVER_ERROR: &str = "Internal Server Error";
pub const MESSAGE_NOT_FOUND: &str = "Not found";


// Bad request messages
#[allow(dead_code)]
pub const MESSAGE_TOKEN_MISSING: &str = "Token is missing";
#[allow(dead_code)]
pub const MESSAGE_BAD_REQUEST: &str = "Bad Request";

pub const EMPTY: &str = "";
