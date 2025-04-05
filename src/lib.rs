/*!
 * Telegrama-rs - A small, reliable Rust library for sending Telegram messages via bots
 *
 * This library provides a simple way to send messages to Telegram chats via bots.
 * It handles formatting, escaping, and error recovery automatically.
 */

pub mod client;
pub mod configuration;
pub mod error;
pub mod formatter;

/// The main entry point for the Telegrama library.
///
/// Provides static methods for configuration and sending messages.
pub struct Telegrama;

impl Telegrama {
    /// Configure the Telegrama library with the provided settings.
    ///
    /// # Examples
    ///
    /// ```
    /// use telegrama_rs::Telegrama;
    ///
    /// Telegrama::configure(|config| {
    ///     config.set_bot_token("YOUR_BOT_TOKEN");
    ///     config.set_chat_id("YOUR_CHAT_ID");
    ///     config.set_default_parse_mode("MarkdownV2");
    /// });
    /// ```
    pub fn configure<F>(config_fn: F)
    where
        F: FnOnce(&mut configuration::Configuration),
    {
        configuration::Configuration::get_instance_mut(config_fn);
    }

    /// Send a message using the configured settings.
    ///
    /// # Arguments
    ///
    /// * `message` - The message text to send
    /// * `options` - Optional parameters to customize the message
    ///
    /// # Returns
    ///
    /// A `Result` containing the response from the Telegram API
    ///
    /// # Examples
    ///
    /// ```
    /// use telegrama_rs::Telegrama;
    ///
    /// // Send a simple message
    /// let result = Telegrama::send_message("Hello from Telegrama-rs!");
    ///
    /// // Send a message with options
    /// let result = Telegrama::send_message(
    ///     "Hello with *formatting*!",
    ///     &[("parse_mode", "MarkdownV2"), ("disable_web_page_preview", "true")]
    /// );
    /// ```
    pub fn send_message<S: AsRef<str>>(
        message: S,
        options: &[(&str, &str)],
    ) -> Result<client::Response, error::Error> {
        // Create a client and send the message
        let client = client::Client::new();
        client.send_message(message.as_ref(), options)
    }
}

// Re-export main components for easy access
pub use client::{Client, Response};
pub use configuration::{ClientOptions, Configuration, FormattingOptions};
pub use error::Error;
pub use formatter::Formatter;

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_library_basics() {
        // Just a simple test to ensure the library compiles
        assert!(true);
    }
}
