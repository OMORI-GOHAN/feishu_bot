mod basic;
mod framework;

use crate::feishu::message::MessageContent;
pub(crate) use framework::CommandRegistry;
use framework::{CommandContext, CommandHandler};

#[derive(Debug)]
pub struct ParsedCommand {
    pub name: String,
    pub args: String,
}

// 组装注册表
pub(crate) fn build_registry() -> anyhow::Result<CommandRegistry> {
    let mut registry = CommandRegistry::new();
    basic::register_commands(&mut registry)?;
    Ok(registry)
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

// 查询注册表，调用对应的处理器
pub async fn execute_command(
    registry: &CommandRegistry,
    client: &mut crate::feishu::client::FeishuClient,
    message: &crate::feishu::message::IncomingMessage,
    command: &ParsedCommand,
) -> anyhow::Result<()> {
    let mut ctx = CommandContext {
        message,
        client,
        registry,
    };

    let contents = match registry.find(&command.name) {
        Some(handler) => handler.handle(&mut ctx, command.args.as_str()).await?,
        None => {
            let message_content = MessageContent::Text {
                text: "未知指令,请使用 help 查看现有指令".to_string(),
            };
            vec![message_content]
        }
    };

    for content in contents {
        ctx.client
            .reply_message(&ctx.message.message_id, &content)
            .await?;
    }

    Ok(())
}

// 依次接收消息并执行指令
pub(crate) async fn process_commands(
    mut message_rx: tokio::sync::mpsc::Receiver<crate::feishu::message::IncomingMessage>,
    feishu_client: &mut crate::feishu::client::FeishuClient,
    shutdown: &tokio_util::sync::CancellationToken,
) -> anyhow::Result<()> {
    // 创建并使用指令注册表
    let registry = crate::command::build_registry()?;

    loop {
        tokio::select! {
            // 同时就绪时优先停止
            biased;

            _ = shutdown.cancelled() => {
                return Ok(());
            }
            item = message_rx.recv() => {
                match item {
                    Some(incoming_message) => {
                        if let Some(command) = crate::command::parse_command(&incoming_message.text) {
                            if let Err(error) = crate::command::execute_command(
                                &registry,
                                feishu_client,
                                &incoming_message,
                                &command)
                            .await {
                                eprintln!("错误: {error}");
                            }
                        }
                    },
                    None => {
                        anyhow::bail!("内部消息通道已关闭");
                    }
                }
            }
        }
    }
}
