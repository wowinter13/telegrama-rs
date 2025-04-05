use crate::error::Error;
use std::sync::{Arc, Mutex, Once};

static INSTANCE: Once = Once::new();
static mut CONFIGURATION: Option<Arc<Mutex<Configuration>>> = None;

/// Formatting options for message processing
#[derive(Debug, Clone)]
pub struct FormattingOptions {
    /// Whether to escape Markdown special characters
    pub escape_markdown: bool,
    /// Whether to obfuscate email addresses in messages
    pub obfuscate_emails: bool,
    /// Whether to escape HTML special characters
    pub escape_html: bool,
    /// Maximum message length (Telegram limit is 4096)
    pub truncate: Option<usize>,
}

impl Default for FormattingOptions {
    fn default() -> Self {
        FormattingOptions {
            escape_markdown: true,
            obfuscate_emails: false,
            escape_html: false,
            truncate: Some(4096),
        }
    }
}

/// HTTP client options for API requests
#[derive(Debug, Clone)]
pub struct ClientOptions {
    /// Request timeout in seconds
    pub timeout: u64,
    /// Number of retries for failed requests
    pub retry_count: u32,
    /// Delay between retries in seconds
    pub retry_delay: u64,
}

impl Default for ClientOptions {
    fn default() -> Self {
        ClientOptions {
            timeout: 30,
            retry_count: 3,
            retry_delay: 1,
        }
    }
}

/// Configuration for the Telegrama client
#[derive(Debug, Clone)]
pub struct Configuration {
    /// Telegram Bot API token
    bot_token: Option<String>,
    /// Default chat ID for sending messages
    chat_id: Option<String>,
    /// Default parse mode (MarkdownV2, HTML, or None)
    default_parse_mode: Option<String>,
    /// Whether to disable web page previews by default
    disable_web_page_preview: bool,
    /// Optional prefix to prepend to all messages
    message_prefix: Option<String>,
    /// Optional suffix to append to all messages
    message_suffix: Option<String>,
    /// Formatting options
    formatting_options: FormattingOptions,
    /// HTTP client options
    client_options: ClientOptions,
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            bot_token: None,
            chat_id: None,
            default_parse_mode: Some("MarkdownV2".to_string()),
            disable_web_page_preview: true,
            message_prefix: None,
            message_suffix: None,
            formatting_options: FormattingOptions::default(),
            client_options: ClientOptions::default(),
        }
    }
}

impl Configuration {
    /// Get a shared reference to the global configuration instance
    ///
    /// # TODO
    ///
    /// This function is unsafe because it uses a static mut reference to the configuration instance.
    #[allow(static_mut_refs)]
    pub fn get_instance() -> Arc<Mutex<Configuration>> {
        unsafe {
            INSTANCE.call_once(|| {
                CONFIGURATION = Some(Arc::new(Mutex::new(Configuration::default())));
            });
            CONFIGURATION.clone().unwrap()
        }
    }

    /// Get a clone of the current configuration (avoids mutex deadlocks)
    pub fn get_cloned_instance() -> Result<Configuration, Error> {
        let instance = Self::get_instance();
        // Use try_lock to avoid deadlocks
        let result = match instance.try_lock() {
            Ok(config) => config.clone(),
            Err(_) => {
                // If we can't get the lock, return a default configuration
                // This is better than deadlocking
                Configuration::default()
            }
        };
        Ok(result)
    }

    /// Modify the global configuration instance
    pub fn get_instance_mut<F>(config_fn: F)
    where
        F: FnOnce(&mut Configuration),
    {
        let instance = Self::get_instance();

        // First try to get the lock
        if let Ok(mut config) = instance.try_lock() {
            config_fn(&mut config);
            return;
        }

        // If we can't get the lock, use a temporary configuration
        let mut temp_config = Configuration::default();
        config_fn(&mut temp_config);

        // Try again to update the global config
        if let Ok(mut config) = Self::get_instance().try_lock() {
            *config = temp_config;
        }
        // If still can't lock, just continue with configuration saved in the closure
    }

    /// Set the Telegram Bot API token
    pub fn set_bot_token<S: AsRef<str>>(&mut self, token: S) {
        self.bot_token = Some(token.as_ref().to_string());
    }

    /// Get the Telegram Bot API token
    pub fn bot_token(&self) -> Result<&str, Error> {
        self.bot_token
            .as_deref()
            .ok_or_else(|| Error::configuration("Bot token not configured"))
    }

    /// Set the default chat ID
    pub fn set_chat_id<S: AsRef<str>>(&mut self, chat_id: S) {
        self.chat_id = Some(chat_id.as_ref().to_string());
    }

    /// Get the default chat ID
    pub fn chat_id(&self) -> Result<&str, Error> {
        self.chat_id
            .as_deref()
            .ok_or_else(|| Error::configuration("Chat ID not configured"))
    }

    /// Set the default parse mode
    pub fn set_default_parse_mode<S: AsRef<str>>(&mut self, mode: S) {
        let mode_str = mode.as_ref().to_string();
        self.default_parse_mode = if mode_str.is_empty() {
            None
        } else {
            Some(mode_str)
        };
    }

    /// Get the default parse mode
    pub fn default_parse_mode(&self) -> Option<&str> {
        self.default_parse_mode.as_deref()
    }

    /// Set whether to disable web page previews by default
    pub fn set_disable_web_page_preview(&mut self, disable: bool) {
        self.disable_web_page_preview = disable;
    }

    /// Get whether web page previews are disabled by default
    pub fn disable_web_page_preview(&self) -> bool {
        self.disable_web_page_preview
    }

    /// Set the message prefix
    pub fn set_message_prefix<S: AsRef<str>>(&mut self, prefix: S) {
        let prefix_str = prefix.as_ref().to_string();
        self.message_prefix = if prefix_str.is_empty() {
            None
        } else {
            Some(prefix_str)
        };
    }

    /// Get the message prefix
    pub fn message_prefix(&self) -> Option<&str> {
        self.message_prefix.as_deref()
    }

    /// Set the message suffix
    pub fn set_message_suffix<S: AsRef<str>>(&mut self, suffix: S) {
        let suffix_str = suffix.as_ref().to_string();
        self.message_suffix = if suffix_str.is_empty() {
            None
        } else {
            Some(suffix_str)
        };
    }

    /// Get the message suffix
    pub fn message_suffix(&self) -> Option<&str> {
        self.message_suffix.as_deref()
    }

    /// Set formatting options
    pub fn set_formatting_options(&mut self, options: FormattingOptions) {
        self.formatting_options = options;
    }

    /// Get formatting options
    pub fn formatting_options(&self) -> &FormattingOptions {
        &self.formatting_options
    }

    /// Set client options
    pub fn set_client_options(&mut self, options: ClientOptions) {
        self.client_options = options;
    }

    /// Get client options
    pub fn client_options(&self) -> &ClientOptions {
        &self.client_options
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), Error> {
        // Check required fields
        self.bot_token()?;

        // Validate parse mode if set
        if let Some(mode) = self.default_parse_mode() {
            if mode.is_empty() || (mode != "MarkdownV2" && mode != "HTML") {
                return Err(Error::configuration(format!(
                    "Invalid parse mode: '{}'. Must be 'MarkdownV2' or 'HTML'",
                    mode
                )));
            }
        }

        Ok(())
    }
}
