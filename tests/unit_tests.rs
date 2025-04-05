use telegrama_rs::{Configuration, FormattingOptions, Telegrama};

#[test]
fn test_configuration() {
    // Test basic configuration
    Telegrama::configure(|config| {
        config.set_bot_token("test_token");
        config.set_chat_id("12345");
        config.set_default_parse_mode("MarkdownV2");
        config.set_disable_web_page_preview(true);
        config.set_message_prefix("[Test] ");
        config.set_message_suffix(" --End");

        let formatting = FormattingOptions {
            escape_markdown: true,
            obfuscate_emails: true,
            escape_html: false,
            truncate: Some(1000),
        };
        config.set_formatting_options(formatting);
    });

    // Verify configuration
    let config = Configuration::get_instance();
    let config = config.lock().unwrap();

    assert_eq!(config.bot_token().unwrap(), "test_token");
    assert_eq!(config.chat_id().unwrap(), "12345");
    assert_eq!(config.default_parse_mode().unwrap(), "MarkdownV2");
    assert!(config.disable_web_page_preview());
    assert_eq!(config.message_prefix().unwrap(), "[Test] ");
    assert_eq!(config.message_suffix().unwrap(), " --End");

    let formatting = config.formatting_options();
    assert!(formatting.escape_markdown);
    assert!(formatting.obfuscate_emails);
    assert!(!formatting.escape_html);
    assert_eq!(formatting.truncate, Some(1000));
}

#[test]
fn test_formatter_email_obfuscation() {
    use telegrama_rs::formatter::Formatter;

    // Test email obfuscation
    let text = "Contact us at info@example.com or john.doe@example.org";
    let obfuscated = Formatter::obfuscate_emails(text);

    assert!(obfuscated.contains("inf...o@example.com"));
    assert!(obfuscated.contains("joh...e@example.org"));
    assert!(!obfuscated.contains("info@example.com"));
    assert!(!obfuscated.contains("john.doe@example.org"));
}
