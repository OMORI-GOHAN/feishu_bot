use crate::feishu::message::MessageContent;

#[derive(Debug)]
pub struct ParsedCommand {
    pub name: String,
    pub args: String,
}

// 解析指令参数
pub fn parse_command(text: &str) -> Option<ParsedCommand> {
    match text.trim_start().split_once(char::is_whitespace) {
        // 丢弃@[Bot_Name]标识符
        Some((_, remaining)) => {
            let remaining = remaining.trim_start();
            if remaining.is_empty() {
                return None;
            }
            // 分割指令和参数
            match remaining.split_once(char::is_whitespace) {
                Some((command, args)) => {
                    // 有参指令
                    let parsed_command = ParsedCommand {
                        name: command.to_string(),
                        args: args.trim_start().to_string(),
                    };
                    return Some(parsed_command);
                }
                None => {
                    // 无参指令
                    let parsed_command = ParsedCommand {
                        name: remaining.to_string(),
                        args: "".to_string(),
                    };
                    return Some(parsed_command);
                }
            }
        }
        None => {}
    }
    None
}

pub async fn execute_command(
    client: &mut crate::feishu::client::FeishuClient,
    message_id: &str,
    command: &ParsedCommand,
) -> anyhow::Result<()> {
    match command.name.as_str() {
        "ping" => {
            command_ping(client, message_id).await?;
        }
        "help" => {}
        _ => {
            let message_content = MessageContent::Text {
                text: "未知指令,请使用help查看现有指令".to_string(),
            };
            client.reply_message(message_id, &message_content).await?;
        }
    }

    Ok(())
}

async fn command_ping(
    client: &mut crate::feishu::client::FeishuClient,
    message_id: &str,
) -> anyhow::Result<()> {
    let message_content = MessageContent::Text {
        text: "pong".to_string(),
    };
    client.reply_message(message_id, &message_content).await?;
    Ok(())
}
