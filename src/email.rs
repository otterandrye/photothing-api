use std::sync::Mutex;

pub struct EmailClient {
    sent_messages: Vec<String>,
}

impl EmailClient {
    pub fn new() -> Self {
        EmailClient { sent_messages: vec![] }
    }

    pub fn send_message(&mut self, address: &str, message: &str) -> Result<(), String> {
        info!("Sending message to '{}': '{}'", address, message);
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

pub fn init_emailer() -> Emailer {
    // TODO: config-time hook to do something smarter
    info!("Adding dummy 'EmailClient' placeholder");
    Emailer { client: Mutex::new(EmailClient::new()) }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn dummy_emailer() {
        let mut emailer = EmailClient::new();
        let sent = emailer.send_message("f@b.com", "hi");
        assert!(sent.is_ok());
        assert_eq!(emailer.messages().get(0).unwrap(), "<f@b.com>::[hi]");
    }
}
