
pub trait Emailer {
    // TODO: temporary API for testing
    fn send_message(&mut self, address: &str, message: &str) -> Result<(), String>;
}

pub struct LogOnlyEmailer {
    sent_messages: Vec<String>,
}

impl LogOnlyEmailer {
    pub fn new() -> Self {
        LogOnlyEmailer { sent_messages: vec![] }
    }

    #[cfg(test)]    
    pub fn messages(&self) -> &Vec<String> {
        &self.sent_messages
    }
}

impl Emailer for LogOnlyEmailer {
    fn send_message(&mut self, address: &str, message: &str) -> Result<(), String> {
        info!("Sending message to '{}': '{}'", address, message);
        self.sent_messages.push(format!("<{}>::[{}]", address, message));
        Ok(())
    }
}

pub fn init_emailer() -> Box<Emailer + Sync + Send> {
    info!("Adding dummy 'LogOnlyEmailer' placeholder");
    Box::new(LogOnlyEmailer::new())
}
