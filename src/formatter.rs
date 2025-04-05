use crate::configuration::Configuration;
use crate::error::Error;
use log::{error, trace};
use once_cell::sync::Lazy;
use regex::Regex;

/// Special characters that need escaping in MarkdownV2 format
const MARKDOWN_SPECIAL_CHARS: &[char] = &[
    '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!',
];

/// Regex for identifying email addresses
static EMAIL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap());

/// Regex for identifying HTML entities
static HTML_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"[<>&]").unwrap());

/// Regex for identifying markdown links
static LINK_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap());

/// Formatter for Telegram messages
pub struct Formatter;

impl Formatter {
    /// Main formatting function that applies all configured transformations
    pub fn format(
        text: &str,
        formatting_options: Option<FormattingOptions>,
    ) -> Result<String, Error> {
        trace!("Original message: {}", text);

        // Get the configuration clone to avoid mutex deadlocks
        let config = match Configuration::get_cloned_instance() {
            Ok(cfg) => cfg,
            Err(e) => {
                error!("Failed to get configuration: {}", e);
                return Err(e);
            }
        };

        // Determine formatting options
        let default_options = config.formatting_options();
        let options = formatting_options.unwrap_or(FormattingOptions {
            escape_markdown: default_options.escape_markdown,
            obfuscate_emails: default_options.obfuscate_emails,
            escape_html: default_options.escape_html,
            truncate: default_options.truncate,
        });

        let mut text = text.to_string();

        // Apply prefix and suffix if configured
        if let Some(prefix) = config.message_prefix() {
            text = format!("{}{}", prefix, text);
        }

        if let Some(suffix) = config.message_suffix() {
            text = format!("{}{}", text, suffix);
        }

        // Apply HTML escaping if enabled
        if options.escape_html {
            text = Self::escape_html(&text);
        }

        // Apply email obfuscation if enabled
        if options.obfuscate_emails {
            text = Self::obfuscate_emails(&text);
        }

        // Apply Markdown escaping if enabled
        if options.escape_markdown {
            text = match Self::escape_markdown_v2(&text) {
                Ok(escaped) => escaped,
                Err(e) => {
                    error!("Markdown escaping failed: {}", e);
                    Self::strip_markdown(&text)
                }
            };
        }

        // Apply truncation if enabled
        if let Some(max_length) = options.truncate {
            text = Self::truncate(&text, max_length);
        }

        trace!("Formatted message: {}", text);

        Ok(text)
    }

    /// Escape special characters for MarkdownV2 format while preserving formatting
    pub fn escape_markdown_v2(text: &str) -> Result<String, Error> {
        if text.is_empty() {
            return Ok(String::new());
        }

        // First pre-process links to handle them specially
        let text = Self::pre_process_links(text);

        // Process with state tracking
        let mut result = String::with_capacity(text.len() * 2);

        // State tracking
        let mut in_code_block = false;
        let mut in_pre_block = false;
        let mut in_bold = false;
        let mut in_italic = false;

        // Track consecutive chars to handle pattern like [text](url)
        let mut in_link_text = false;
        let mut in_link_url = false;

        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                // Handle code blocks (no escaping within code blocks)
                '`' => {
                    // Check for triple backtick (code block)
                    if !in_code_block
                        && chars.peek() == Some(&'`')
                        && chars.clone().nth(1) == Some('`')
                    {
                        // Start/end of triple backtick block
                        result.push(c);
                        result.push(chars.next().unwrap()); // consume second backtick
                        result.push(chars.next().unwrap()); // consume third backtick
                        in_pre_block = !in_pre_block;
                        trace!("Triple backtick: in_pre_block={}", in_pre_block);
                    } else {
                        // Single backtick (inline code)
                        result.push(c);
                        in_code_block = !in_code_block;
                        trace!("Single backtick: in_code_block={}", in_code_block);
                    }
                }

                // Handle bold formatting
                '*' if !in_code_block && !in_pre_block => {
                    // Toggle bold state
                    in_bold = !in_bold;
                    result.push(c);
                }

                // Handle italic formatting
                '_' if !in_code_block && !in_pre_block => {
                    // Toggle italic state
                    in_italic = !in_italic;
                    result.push(c);
                }

                // Handle link text start
                '[' if !in_code_block && !in_pre_block && !in_link_text => {
                    in_link_text = true;
                    result.push(c);
                }

                // Handle link text end
                ']' if !in_code_block && !in_pre_block && in_link_text => {
                    in_link_text = false;
                    result.push(c);

                    // Check if followed by opening parenthesis for URL
                    if chars.peek() == Some(&'(') {
                        in_link_url = true;
                    }
                }

                // Handle link URL start
                '(' if !in_code_block && !in_pre_block && !in_link_text && in_link_url => {
                    result.push(c);
                }

                // Handle link URL end
                ')' if !in_code_block && !in_pre_block && in_link_url => {
                    in_link_url = false;
                    result.push(c);
                }

                // Escape special characters outside formatting contexts
                _ if !in_code_block
                    && !in_pre_block
                    && !in_bold
                    && !in_italic
                    && !in_link_text
                    && !in_link_url
                    && MARKDOWN_SPECIAL_CHARS.contains(&c) =>
                {
                    result.push('\\');
                    result.push(c);
                    trace!("Escaped character: \\{}", c);
                }

                // Regular character or character within a formatting context
                _ => {
                    result.push(c);
                }
            }
        }
        Ok(result)
    }

    /// Pre-process links to handle them as special entities
    fn pre_process_links(text: &str) -> String {
        // This method will identify complete markdown links [text](url) and handle them specially
        let mut result = text.to_string();

        // First process complete markdown links
        result = LINK_REGEX
            .replace_all(&result, |caps: &regex::Captures| {
                let link_text = &caps[1];
                let url = &caps[2];

                // Escape special characters in the link text
                let escaped_text = link_text
                    .chars()
                    .map(|c| {
                        if MARKDOWN_SPECIAL_CHARS.contains(&c) {
                            format!("\\{}", c)
                        } else {
                            c.to_string()
                        }
                    })
                    .collect::<String>();

                // For the URL, don't escape the protocol part
                let mut escaped_url = url.to_string();
                for ch in MARKDOWN_SPECIAL_CHARS {
                    if *ch != '/' && *ch != ':' && *ch != '.' && *ch != '-' {
                        escaped_url = escaped_url.replace(*ch, &format!("\\{}", ch));
                    }
                }

                format!("[{}]({})", escaped_text, escaped_url)
            })
            .to_string();

        result
    }

    /// Strip all Markdown formatting from text
    pub fn strip_markdown(text: &str) -> String {
        // Remove bold, italic, code, and links
        let no_format = text.replace(['*', '_', '`'], "");

        // Remove links in format [text](url)
        let link_regex = Regex::new(r"\[([^\]]+)\]\([^)]+\)").unwrap_or_else(|_| {
            error!("Failed to compile link regex");
            Regex::new(r"").unwrap()
        });

        let result = link_regex.replace_all(&no_format, "$1").to_string();

        result
    }

    /// Obfuscate email addresses in text
    pub fn obfuscate_emails(text: &str) -> String {
        let result = EMAIL_REGEX
            .replace_all(text, |caps: &regex::Captures| {
                let email = &caps[0];
                let parts: Vec<&str> = email.split('@').collect();

                if parts.len() != 2 {
                    trace!("Not a standard email format: {}", email);
                    return email.to_string();
                }

                let username = parts[0];
                let domain = parts[1];

                if username.len() <= 3 {
                    trace!("Username too short to obfuscate: {}", username);
                    return email.to_string();
                }

                // Keep first three chars and last one of username
                let visible_prefix = &username[0..3];
                let visible_suffix = &username[username.len() - 1..];

                let result = format!("{}...{}@{}", visible_prefix, visible_suffix, domain);
                trace!("Obfuscated email: {} -> {}", email, result);

                result
            })
            .to_string();

        result
    }

    /// Escape HTML special characters
    pub fn escape_html(text: &str) -> String {
        let result = HTML_REGEX
            .replace_all(text, |caps: &regex::Captures| match &caps[0] {
                "<" => {
                    trace!("Escaped HTML: < -> &lt;");
                    "&lt;".to_string()
                }
                ">" => {
                    trace!("Escaped HTML: > -> &gt;");
                    "&gt;".to_string()
                }
                "&" => {
                    trace!("Escaped HTML: & -> &amp;");
                    "&amp;".to_string()
                }
                _ => caps[0].to_string(),
            })
            .to_string();

        result
    }

    /// Truncate text to a maximum length
    pub fn truncate(text: &str, max_length: usize) -> String {
        if text.len() <= max_length {
            return text.to_string();
        }

        // Try to truncate at a space to avoid cutting words
        if let Some(last_space) = text[..max_length].rfind(' ') {
            let result = format!("{}...", &text[..last_space]);
            result
        } else {
            let result = format!("{}...", &text[..max_length - 3]);
            result
        }
    }
}

/// Options for message formatting
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
