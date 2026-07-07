use crate::env::HerdrEnv;

/// Minimal runtime logger for plugin code.
pub struct Logger<'a> {
    env: &'a HerdrEnv,
}

impl<'a> Logger<'a> {
    pub(crate) fn new(env: &'a HerdrEnv) -> Self {
        Self { env }
    }

    pub fn info(&self, message: impl AsRef<str>) {
        self.write("INFO", message.as_ref());
    }

    pub fn warn(&self, message: impl AsRef<str>) {
        self.write("WARN", message.as_ref());
    }

    pub fn error(&self, message: impl AsRef<str>) {
        self.write("ERROR", message.as_ref());
    }

    fn write(&self, level: &str, message: &str) {
        let plugin_id = self.env.plugin_id.as_deref().unwrap_or("herdr-plugin");
        eprintln!("[{level}] {plugin_id}: {message}");
    }
}
