use log::{error, info};
use reqwest::blocking::{Client as ReqwestClient, Response as ReqwestResponse};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::configuration::Configuration;
use crate::error::Error;
use crate::formatter::Formatter;
use crate::formatter::FormattingOptions;

/// HTTP client for communicating with the Telegram API
pub struct Client {
    client: ReqwestClient,
}

/// Response from the Telegram API
#[derive(Debug, Deserialize)]
pub struct Response {
    /// Whether the request was successful
    pub ok: bool,
    /// Response description
    pub description: Option<String>,
    /// Response result
    pub result: Option<serde_json::Value>,
}

/// Helper struct for building API requests
#[derive(Debug, Serialize)]
struct SendMessageParams<'a> {
    /// Telegram chat ID
    chat_id: &'a str,
    /// Message text
    text: &'a str,
    /// Parse mode (MarkdownV2, HTML, or None)
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_mode: Option<&'a str>,
    /// Whether to disable web page previews
    #[serde(skip_serializing_if = "Option::is_none")]
    disable_web_page_preview: Option<bool>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    /// Create a new Telegram client
    pub fn new() -> Self {
        let client = ReqwestClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| ReqwestClient::new());

        Client { client }
    }

    /// Send a message to the Telegram API
    pub fn send_message(&self, message: &str, options: &[(&str, &str)]) -> Result<Response, Error> {
        // Get configuration using the clone method to avoid deadlocks
        let config = Configuration::get_cloned_instance()?;

        // Validate configuration
        config.validate()?;

        // Get required parameters
        let bot_token = config.bot_token()?;

        // Determine chat ID (options override config)
        let chat_id = options
            .iter()
            .find(|(k, _)| *k == "chat_id")
            .map(|(_, v)| *v)
            .unwrap_or_else(|| {
                // Use default chat ID as fallback
                match config.chat_id() {
                    Ok(id) => id,
                    Err(_) => {
                        // This shouldn't happen due to validation, but handle it anyway
                        error!("No chat ID provided in options or configuration");
                        ""
                    }
                }
            });

        if chat_id.is_empty() {
            return Err(Error::configuration("Chat ID not provided"));
        }

        // Get default formatting options from the config
        let config_formatting = config.formatting_options();

        // Convert config formatting options to formatter module's FormattingOptions
        let default_formatting_options = crate::formatter::FormattingOptions {
            escape_markdown: config_formatting.escape_markdown,
            obfuscate_emails: config_formatting.obfuscate_emails,
            escape_html: config_formatting.escape_html,
            truncate: config_formatting.truncate,
        };

        // Extract formatting options from options
        let formatting_options =
            self.extract_formatting_options(options, default_formatting_options);

        // Extract parse mode from options (or use default)
        let mut parse_mode = options
            .iter()
            .find(|(k, _)| *k == "parse_mode")
            .map(|(_, v)| *v)
            .or_else(|| config.default_parse_mode());

        // Validate parse mode
        if let Some(mode) = parse_mode {
            if mode != "MarkdownV2" && mode != "HTML" && !mode.is_empty() {
                // Fix invalid parse mode to avoid API errors
                parse_mode = None;
            }
        }

        // Extract web page preview setting from options (or use default)
        let disable_web_page_preview = options
            .iter()
            .find(|(k, _)| *k == "disable_web_page_preview")
            .map(|(_, v)| v.to_lowercase() == "true")
            .unwrap_or_else(|| config.disable_web_page_preview());

        // Format the message
        info!("Formatting message: {}", message);

        // Apply different formatting based on parse mode
        let formatted_message = match parse_mode {
            Some("MarkdownV2") => {
                // Use markdown formatting
                let md_formatting_options = FormattingOptions {
                    escape_markdown: true,
                    escape_html: false,
                    obfuscate_emails: formatting_options.obfuscate_emails,
                    truncate: formatting_options.truncate,
                };
                Formatter::format(message, Some(md_formatting_options))?
            }
            Some("HTML") => {
                // Use HTML formatting
                let html_formatting_options = FormattingOptions {
                    escape_markdown: false,
                    escape_html: true,
                    obfuscate_emails: formatting_options.obfuscate_emails,
                    truncate: formatting_options.truncate,
                };
                Formatter::format(message, Some(html_formatting_options))?
            }
            _ => {
                // Plain text, no special formatting
                let plain_formatting_options = FormattingOptions {
                    escape_markdown: false,
                    escape_html: false,
                    obfuscate_emails: formatting_options.obfuscate_emails,
                    truncate: formatting_options.truncate,
                };
                Formatter::format(message, Some(plain_formatting_options))?
            }
        };

        info!("Formatted message: {}", formatted_message);

        // Handle empty messages
        if formatted_message.is_empty() {
            error!("Message is empty after formatting");
            return Err(Error::formatting("Message is empty after formatting"));
        }

        let result = self.send_message_request(
            bot_token,
            chat_id,
            &formatted_message,
            parse_mode,
            disable_web_page_preview,
        );

        // Try fallbacks if primary fails
        match result {
            Ok(response) => {
                info!("Message sent successfully!");
                Ok(response)
            }
            Err(e) => {
                // Log the error
                error!("Error sending message with primary parse mode: {}", e);

                // Extract status code if it's an API error
                let should_try_plaintext = match &e {
                    Error::Api(desc) => desc.contains("parse_mode"),
                    _ => false,
                };

                if should_try_plaintext {
                    // Try with plain text as fallback
                    info!("Falling back to plain text format");

                    // Use the original message with minimal formatting
                    let plain_formatting_options = FormattingOptions {
                        escape_markdown: false,
                        escape_html: false,
                        obfuscate_emails: formatting_options.obfuscate_emails,
                        truncate: formatting_options.truncate,
                    };

                    let plain_message = Formatter::format(message, Some(plain_formatting_options))?;

                    self.send_message_request(
                        bot_token,
                        chat_id,
                        &plain_message,
                        None,
                        disable_web_page_preview,
                    )
                } else if parse_mode == Some("MarkdownV2") {
                    // Try with HTML as fallback
                    info!("Falling back to HTML format");

                    // Format message for HTML
                    let html_formatting_options = FormattingOptions {
                        escape_markdown: false,
                        escape_html: true,
                        obfuscate_emails: formatting_options.obfuscate_emails,
                        truncate: formatting_options.truncate,
                    };

                    let html_message = Formatter::format(message, Some(html_formatting_options))?;

                    match self.send_message_request(
                        bot_token,
                        chat_id,
                        &html_message,
                        Some("HTML"),
                        disable_web_page_preview,
                    ) {
                        Ok(response) => Ok(response),
                        Err(html_error) => {
                            // If HTML fails too, try plain text
                            error!("Error sending message with HTML format: {}", html_error);
                            info!("Falling back to plain text format");

                            // Format message as plain text
                            let plain_formatting_options = FormattingOptions {
                                escape_markdown: false,
                                escape_html: false,
                                obfuscate_emails: formatting_options.obfuscate_emails,
                                truncate: formatting_options.truncate,
                            };

                            let plain_message =
                                Formatter::format(message, Some(plain_formatting_options))?;

                            self.send_message_request(
                                bot_token,
                                chat_id,
                                &plain_message,
                                None,
                                disable_web_page_preview,
                            )
                        }
                    }
                } else {
                    // Return the original error
                    Err(e)
                }
            }
        }
    }

    /// Send a request to the Telegram API
    fn send_message_request(
        &self,
        bot_token: &str,
        chat_id: &str,
        text: &str,
        parse_mode: Option<&str>,
        disable_web_page_preview: bool,
    ) -> Result<Response, Error> {
        // Build the API URL
        let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

        // Handle parse_mode - Telegram API requires empty string or a valid mode, not null
        // Based on API testing, null is not accepted but empty string is
        #[allow(clippy::redundant_guards)]
        let effective_parse_mode = match parse_mode {
            Some(mode) if mode.is_empty() => Some(""),
            Some(mode) if mode != "MarkdownV2" && mode != "HTML" => Some(""),
            None => Some(""), // Use empty string instead of None (null)
            other => other,
        };

        // Prepare parameters
        let params = SendMessageParams {
            chat_id,
            text,
            parse_mode: effective_parse_mode,
            disable_web_page_preview: Some(disable_web_page_preview),
        };

        let response = match self.client.post(&url).json(&params).send() {
            Ok(resp) => {
                info!("Received response with status code: {}", resp.status());

                // Read the response body as text for debugging
                match resp.text() {
                    Ok(_) => {
                        // We need to re-send the request since we've consumed the body
                        match self.client.post(&url).json(&params).send() {
                            Ok(new_resp) => new_resp,
                            Err(e) => {
                                return Err(Error::Http(e));
                            }
                        }
                    }
                    Err(_) => {
                        // We need to re-send the request since we've consumed the response
                        match self.client.post(&url).json(&params).send() {
                            Ok(new_resp) => new_resp,
                            Err(e) => {
                                return Err(Error::Http(e));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("HTTP request failed: {}", e);
                return Err(Error::Http(e));
            }
        };

        // Parse the response
        info!("Processing response...");
        self.handle_response(response)
    }

    /// Handle the API response
    fn handle_response(&self, response: ReqwestResponse) -> Result<Response, Error> {
        // Check HTTP status code
        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .unwrap_or_else(|_| "Unable to read response body".to_string());

            error!("HTTP error status {}: {}", status.as_u16(), body);

            return Err(Error::api(format!(
                "HTTP error (status {}): {}",
                status.as_u16(),
                body
            )));
        }

        // Parse the response as JSON
        info!("Parsing response as JSON...");
        let telegram_response: Response = match response.json() {
            Ok(res) => {
                info!("Successfully parsed response JSON");
                res
            }
            Err(e) => {
                error!("Failed to parse API response: {}", e);
                return Err(Error::api(format!("Failed to parse API response: {}", e)));
            }
        };

        // Check if the API returned an error
        if !telegram_response.ok {
            let description = telegram_response
                .description
                .unwrap_or_else(|| "Unknown API error".to_string());

            error!("Telegram API returned error: {}", description);
            return Err(Error::api(description));
        }

        info!("Request was successful (ok=true)");
        Ok(telegram_response)
    }

    /// Extract formatting options from the options array
    fn extract_formatting_options(
        &self,
        options: &[(&str, &str)],
        default_formatting_options: FormattingOptions,
    ) -> FormattingOptions {
        // Start with defaults
        let mut formatting_options = default_formatting_options;

        // Override with any provided options
        for (key, value) in options {
            match *key {
                "escape_markdown" => {
                    formatting_options.escape_markdown = value.to_lowercase() == "true";
                }
                "obfuscate_emails" => {
                    formatting_options.obfuscate_emails = value.to_lowercase() == "true";
                }
                "escape_html" => {
                    formatting_options.escape_html = value.to_lowercase() == "true";
                }
                "truncate" => {
                    formatting_options.truncate = value.parse::<usize>().ok();
                }
                _ => {}
            }
        }

        formatting_options
    }
}
