# 💬 `telegrama-rs` – A small Rust library for Telegram messages

**Telegrama-rs** is a minimalist, reliable Rust wrapper for sending admin/notification messages via Telegram bots.

Shoutout to [rameerez](https://x.com/rameerez) for the original idea and Ruby implementation. Usually I am not proud of copying things "as is" to another language, but this time it was really helpful.

## Features

- **Simple API**: Focus on ease of use with a minimal, intuitive interface
- **Robust Error Handling**: Multi-level fallback system to ensure messages are delivered
- **Formatting**: Handles Markdown, HTML, and text formatting with automatic escaping
- **Email Obfuscation**: Optional privacy protection for emails in messages
- **Configuration**: Flexible defaults with per-message overrides

## Installation

Add telegrama-rs to your Cargo.toml:

```toml
[dependencies]
telegrama-rs = "0.1.0"
```

## Quick Start

```rust
use telegrama_rs::Telegrama;

fn main() {
    // Configure the client
    Telegrama::configure(|config| {
        config.set_bot_token("YOUR_BOT_TOKEN");
        config.set_chat_id("YOUR_CHAT_ID");
    });
    
    // Send a simple message
    match Telegrama::send_message("Hello from Telegrama-rs!") {
        Ok(_) => println!("Message sent successfully"),
        Err(e) => eprintln!("Failed to send message: {}", e),
    }
    
    // Send a formatted message
    let message = format!(
        "🚨 *Alert*: System {}\n\nCPU usage: *{}%*\nMemory: *{}%*\n\n[View Dashboard]({})",
        "production-db-1",
        95,
        87,
        "https://dashboard.example.com"
    );
    
    Telegrama::send_message(message).unwrap();
}
```

## Configuration

Configure the client with your bot token and chat ID:

```rust
Telegrama::configure(|config| {
    // Required settings
    config.set_bot_token("YOUR_BOT_TOKEN");
    config.set_chat_id("YOUR_CHAT_ID");
    
    // Optional settings
    config.set_default_parse_mode("MarkdownV2"); // or "HTML"
    config.set_disable_web_page_preview(true);
    
    // Message prefix/suffix (for identifying source)
    config.set_message_prefix("[MyApp] ");
    config.set_message_suffix("\n--\nSent from MyApp");
    
    // Formatting options
    let formatting = FormattingOptions {
        escape_markdown: true,
        obfuscate_emails: true,
        escape_html: false,
        truncate: Some(4096),
    };
    config.set_formatting_options(formatting);
    
    // HTTP client options
    let client_options = ClientOptions {
        timeout: 30,
        retry_count: 3,
        retry_delay: 1,
    };
    config.set_client_options(client_options);
});
```

## Advanced Usage

### Override Options Per Message

You can override configuration defaults when sending individual messages:

```rust
// Override chat ID and formatting options
Telegrama::send_message(
    "Message to a different chat",
    &[
        ("chat_id", "DIFFERENT_CHAT_ID"),
        ("parse_mode", "HTML"),
        ("disable_web_page_preview", "false"),
        ("escape_markdown", "false"),
        ("obfuscate_emails", "true"),
    ],
).unwrap();
```

### Email Obfuscation

For privacy protection, you can obfuscate email addresses in messages:

```rust
// Configure globally
Telegrama::configure(|config| {
    let mut formatting = FormattingOptions::default();
    formatting.obfuscate_emails = true;
    config.set_formatting_options(formatting);
});

// Or per message
Telegrama::send_message(
    "User john.doe@example.com registered", 
    &[("obfuscate_emails", "true")]
).unwrap();
// Sends: "User joh...e@example.com registered"
```

## License

This project is licensed under the MIT License - see the LICENSE file for details. 
