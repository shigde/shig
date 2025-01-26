pub mod activate_account;
pub mod forgotten_password;

#[allow(dead_code)]
pub trait Template {
    fn render(&self) -> Result<String, handlebars::RenderError>;
    fn get_subject(&self) -> String;
}
