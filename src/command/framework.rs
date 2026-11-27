use crate::feishu::{client::FeishuClient, message::IncomingMessage, message::MessageContent};
use async_trait::async_trait;

pub struct CommandContext<'a> {
    pub message: &'a IncomingMessage,
    pub client: &'a mut FeishuClient,
    pub registry: &'a CommandRegistry, // 查询指令与生成帮助
}

// 处理器接口必须实现以下方法：名字，用途，处理行为; 除此以外还默认了两个方法，别名和是否需要管理员权限
#[async_trait]
pub trait CommandHandler: Send + Sync {
    fn name(&self) -> &'static str;

    fn usage(&self) -> &'static str;

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn admin_only(&self) -> bool {
        false
    }

    async fn handle(
        &self,
        ctx: &mut CommandContext<'_>,
        args: &str,
    ) -> anyhow::Result<Vec<MessageContent>>;
}

// 指令注册表
pub struct CommandRegistry {
    handlers: Vec<Box<dyn CommandHandler>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn find(&self, name: &str) -> Option<&dyn CommandHandler> {
        for handler in &self.handlers {
            if handler.name() == name || handler.aliases().contains(&name) {
                return Some(handler.as_ref()); // 返回处理器，解开Box
            }
        }
        None
    }

    pub fn help_text(&self) -> String {
        let mut lines = Vec::new();
        for handler in &self.handlers {
            lines.push(handler.usage());
        }
        lines.join("\n\n")
    }

    // 检查名称冲突并保存处理器
    pub fn register(&mut self, handler: Box<dyn CommandHandler>) -> anyhow::Result<()> {
        // 将新处理器所有名称存入一个vector
        let mut names = vec![handler.name()];
        names.extend_from_slice(handler.aliases());

        // 遍历检查
        for name in names {
            if !self.find(name).is_none() {
                anyhow::bail!("已有同名处理器");
            }
        }
        self.handlers.push(handler);

        Ok(())
    }
}
