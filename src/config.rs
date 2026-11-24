use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
pub struct Config {
    pub feishu: FeishuConfig,
    pub rabbitmq: RabbitMqConfig,
    pub bot: BotConfig,
}

#[derive(Deserialize)]
pub struct FeishuConfig {
    pub app_id: String,
    pub app_secret: String,
}

#[derive(Deserialize)]
pub struct RabbitMqConfig {
    pub url: String,
    pub queue: String,
}

#[derive(Deserialize)]
pub struct BotConfig {
    pub target_chat_id: String,
    pub admin_open_ids: Vec<String>,
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        // 加载配置
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        // 检查字段空值
        if self.feishu.app_id.trim().is_empty() {
            anyhow::bail!("feishu.app_id 不能为空");
        }

        if self.feishu.app_secret.trim().is_empty() {
            anyhow::bail!("feishu.app_secret 不能为空");
        }

        if self.rabbitmq.url.trim().is_empty() {
            anyhow::bail!("rabbitmq.url 不能为空");
        }

        if self.rabbitmq.queue.trim().is_empty() {
            anyhow::bail!("rabbitmq.queue 不能为空");
        }

        if self.bot.target_chat_id.trim().is_empty() {
            anyhow::bail!("bot.target_chat_id 不能为空");
        }

        Ok(())
    }
}
