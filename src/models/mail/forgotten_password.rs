use crate::models::mail::template::forgotten_password::FORGOTTEN_PASSWORD;
use crate::models::mail::template::Template;
use handlebars::Handlebars;

pub struct ForgottenPassword {
    user: String,
    link: String,
    instance: String,
    template_name: String,
    subject: String,
}

impl ForgottenPassword {
    pub fn new(user: String, link: String, instance: String) -> Self {
        ForgottenPassword {
            user,
            link,
            instance,
            template_name: String::from("forgotten_password"),
            subject: String::from("Your password reset token (valid for only 10 minutes)"),
        }
    }
}

impl Template for ForgottenPassword {
    fn render(&self) -> Result<String, handlebars::RenderError> {
        let template_name = self.template_name.as_str();
        let mut handlebars = Handlebars::new();
        handlebars.register_template_string(template_name, FORGOTTEN_PASSWORD)?;

        let data = serde_json::json!({
            "user": &self.user,
            "link": &self.link,
            "instance": &self.instance,
        });

        let content_template = handlebars.render(template_name, &data)?;

        Ok(content_template)
    }

    fn get_subject(&self) -> String {
        self.subject.clone()
    }
}
