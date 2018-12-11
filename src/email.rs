use mailgun_v3::Credentials;
use mailgun_v3::email::{Message, MessageBody, EmailAddress, send_email};
use reqwest::Error;
use std::sync::Mutex;

pub struct EmailClient {
    sent_messages: Vec<String>,
    creds: Option<Credentials>,
    from: EmailAddress,
    pub app_url: String,
}

impl EmailClient {
    pub fn new() -> Self {
        EmailClient {
            sent_messages: vec![],
            creds: None,
            from: EmailAddress::address(""),
            app_url: "".to_owned(),
        }
    }

    pub fn configured(system_email: &str, app_url: &str, mailgun_creds: Credentials) -> Self {
        let mut default = EmailClient::new();
        default.from = EmailAddress::address(&system_email);
        default.creds = Some(mailgun_creds);
        default.app_url = app_url.to_owned();
        default
    }

    pub fn send_message(&mut self, address: &str, message: &str) -> Result<(), Error> {
        info!("Sending message to '{}': '{}'", address, message);

        if let Some(ref creds) = self.creds {
            info!("   Mailgun configured, sending actual email...");
            let msg = Message {
                to: vec![EmailAddress::address(address)],
                subject: format!("Password Reset Request"),
                body: MessageBody::Text(message.to_owned()),
                ..Default::default()
            };
            send_email(creds, &self.from, msg)?;
        }

        self.sent_messages.push(format!("<{}>::[{}]", address, message));
        Ok(())
    }

    #[cfg(test)]
    pub fn messages(&self) -> &Vec<String> {
        &self.sent_messages
    }
}

pub struct Emailer {
    pub client: Mutex<EmailClient>
}

pub fn dummy_emailer() -> Emailer {
    info!("Adding dummy 'EmailClient' placeholder");
    Emailer { client: Mutex::new(EmailClient::new()) }
}

pub fn init_emailer(api_key: &str, domain: &str, system_email: &str, app_url: &str) -> Emailer {
    info!("Adding configured mailgun instance");
    let creds = Credentials::new(&api_key, &domain);
    Emailer { client: Mutex::new(EmailClient::configured(system_email, app_url, creds)) }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[ignore]
    fn dummy_emailer() {
        let mut emailer = EmailClient::new();
        let sent = emailer.send_message("f@b.com", "hi");
        assert!(sent.is_ok());
        assert_eq!(emailer.messages().get(0).unwrap(), "<f@b.com>::[hi]");
    }
}
