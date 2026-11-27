use super::framework::{CommandContext, CommandHandler};
use crate::feishu::message::MessageContent;
use async_trait::async_trait;

pub struct PingCommand;
pub struct HelpCommand;

// 所有指令增加时在这里注册
pub fn register_commands(registry: &mut super::framework::CommandRegistry) -> anyhow::Result<()> {
    registry.register(Box::new(PingCommand))?;
    registry.register(Box::new(HelpCommand))?;

    Ok(())
}

#[async_trait]
impl CommandHandler for PingCommand {
    fn name(&self) -> &'static str {
        "ping"
    }

    fn usage(&self) -> &'static str {
        "ping: 检查机器人是否在线"
    }

    async fn handle(
        &self,
        _ctx: &mut CommandContext<'_>,
        _args: &str,
    ) -> anyhow::Result<Vec<MessageContent>> {
        let message_content = MessageContent::Text {
            text: "pong".to_string(),
        };
        Ok(vec![message_content])
    }
}

#[async_trait]
impl CommandHandler for HelpCommand {
    fn name(&self) -> &'static str {
        "帮助"
    }

    fn usage(&self) -> &'static str {
        "帮助 / help: 查看指令列表"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["help"]
    }

    async fn handle(
        &self,
        ctx: &mut CommandContext<'_>,
        _args: &str,
    ) -> anyhow::Result<Vec<MessageContent>> {
        let text = ctx.registry.help_text();
        let message_content = MessageContent::Text { text };
        Ok(vec![message_content])
    }
}
