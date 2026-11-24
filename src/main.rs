mod config;
mod feishu;
mod queue;

use config::Config;
use feishu::client::FeishuClient;
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载各项配置
    let config = Config::load(Path::new("config.toml"))?;
    println!("配置加载成功");

    // 创建客户端
    let mut feishu_client = FeishuClient::new(config.feishu)?;

    // 获取 RabbitMQ 连接与通道
    let (_connection, channel) = queue::connect(&config.rabbitmq).await?;

    // 创建消费者并消费消息队列
    let consumer = queue::create_consumer(&channel, &config.rabbitmq.queue).await?;
    println!("开始等待消息队列");
    queue::consume(consumer, &mut feishu_client, &config.bot.target_chat_id).await?;

    Ok(())
}
