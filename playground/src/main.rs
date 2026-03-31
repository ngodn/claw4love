mod config;
mod message;
mod permissions;

use message::*;
use permissions::*;

fn main() {
    println!("=== claw4love playground ===\n");

    // Test message creation
    let msg = Message::User(UserMessage {
        uuid: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        message: UserMessageContent {
            role: "user".into(),
            content: ContentBlock::Text("Hello, claw4love!".into()),
        },
        is_meta: None,
        is_compact_summary: None,
        origin: None,
    });
    let json = serde_json::to_string_pretty(&msg).unwrap();
    println!("User message:\n{json}\n");

    // Test permission context
    let ctx = ToolPermissionContext {
        always_allow_rules: vec![ToolPermissionRule {
            tool_name: "BashTool".into(),
            pattern: "git".into(),
        }],
        ..Default::default()
    };

    let result = ctx.check("BashTool", "git status");
    println!("Permission check (git status): {result:?}");

    let result = ctx.check("BashTool", "rm -rf /");
    println!("Permission check (rm -rf /): {result:?}");

    // Test config
    let config = config::C4lConfig::default();
    println!("\nDefault model: {}", config.model.default_model);
    println!("API base URL: {}", config.api_base_url());

    let toml_str = toml::to_string_pretty(&config).unwrap();
    println!("\nDefault config.toml:\n{toml_str}");
}
