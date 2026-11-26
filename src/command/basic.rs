use super::framework::{CommandContext, CommandHandler};
use crate::feishu::message::MessageContent;
use async_trait::async_trait;

pub struct PingCommand;

// 所有指令增加时在这里注册
pub fn register_commands(
    registry: &mut super::framework::CommandRegistry,
) -> anyhow::Result<()> {
    registry.register(Box::new(PingCommand))
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
        let message_content = MessageContent::Text { text: "pong".to_string() };
        Ok(vec![message_content])
    }
}

