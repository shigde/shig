use crate::models::mail::template::Template;
use handlebars::Handlebars;

pub struct ActivateAccount {
    user: String,
    link: String,
    instance: String,
    template_name: String,
    subject: String
}

impl ActivateAccount {
    pub fn new(user: String, link: String, instance: String) -> Self {
        ActivateAccount {
            user,
            link,
            instance,
            template_name: String::from("activate_account"),
            subject: String::from("Your account verification code"),
        }
    }
}

impl Template for ActivateAccount {
    fn render(&self) -> Result<String, handlebars::RenderError> {
        let template_name = self.template_name.as_str();
        let mut handlebars = Handlebars::new();
        handlebars.register_template_file(
            template_name,
            &format!("./templates/{}.hbs", self.template_name),
        )?;

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
