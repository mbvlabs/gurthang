use askama::Template;
use gurthang::prelude::*;
use gurthang::Result;

pub struct AuthMailer;

impl Mailer for AuthMailer {}

#[derive(Template)]
#[template(path = "auth/welcome.html")]
struct WelcomeHtml<'a> {
    email: &'a str,
}

#[derive(Template)]
#[template(path = "auth/welcome.txt")]
struct WelcomeText<'a> {
    email: &'a str,
}

impl AuthMailer {
    pub async fn send_welcome(ctx: &Context, email: &str) -> Result<()> {
        let html = WelcomeHtml { email }
            .render()
            .map_err(|error| Error::Message(error.to_string()))?;
        let text = WelcomeText { email }
            .render()
            .map_err(|error| Error::Message(error.to_string()))?;
        Self::mail(
            ctx,
            &Email {
                to: email.to_owned(),
                subject: "Welcome".into(),
                html,
                text,
                ..Default::default()
            },
        )
        .await
    }
}
