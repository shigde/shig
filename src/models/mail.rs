use crate::models::mail::config::MailConfig;
use crate::models::mail::forgotten_password::ForgottenPassword;
use crate::models::mail::template::Template;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use crate::models::mail::activate_account::ActivateAccount;

mod activate_account;
mod config;
mod forgotten_password;
mod template;

#[allow(dead_code)]
pub struct Email {
    to_user: String,
    to_email: String,
    from: String,
    config: MailConfig,
    template: Box<dyn Template>,
}

impl Email {
    fn new(user: String, email: String, config: MailConfig, tmpl: impl Template + 'static) -> Self {
        let from = format!("Shig <{}>", config.smtp_user.to_owned());

        Email {
            to_user: user,
            to_email: email,
            from,
            config,
            template: Box::new(tmpl),
        }
    }

    #[allow(dead_code)]
    pub fn new_forgotten_password(
        user: String,
        email: String,
        link: String,
        inst: String,
        config: MailConfig,
    ) -> Self {
        let tpl = ForgottenPassword::new(user.clone(), link.clone(), inst);

        Self::new(user, email, config, tpl)
    }

    #[allow(dead_code)]
    pub fn new_activate_account(
        user: String,
        email: String,
        link: String,
        inst: String,
        config: MailConfig,
    ) -> Self {
        let tpl = ActivateAccount::new(user.clone(), link.clone(), inst);

        Self::new(user, email, config, tpl)
    }

    #[allow(dead_code)]
    fn new_transport(
        &self,
    ) -> Result<AsyncSmtpTransport<Tokio1Executor>, lettre::transport::smtp::Error> {
        let creds = Credentials::new(
            self.config.smtp_user.to_owned(),
            self.config.smtp_pass.to_owned(),
        );

        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(
            &self.config.smtp_host.to_owned(),
        )?
        .port(self.config.smtp_port)
        .credentials(creds)
        .build();

        Ok(transport)
    }

    #[allow(dead_code)]
    async fn send_email(&self) -> Result<(), Box<dyn std::error::Error>> {
        let html_template = self.template.render()?;

        let email = Message::builder()
            .to(
                format!("{} <{}>", self.to_user.as_str(), self.to_email.as_str())
                    .parse()
                    .unwrap(),
            )
            .reply_to(self.from.as_str().parse().unwrap())
            .from(self.from.as_str().parse().unwrap())
            .subject(self.template.get_subject())
            .header(ContentType::TEXT_HTML)
            .body(html_template)?;

        let transport = self.new_transport()?;

        transport.send(email).await?;
        Ok(())
    }
}
