mod config;
mod feishu;

use config::Config;
use std::path::Path;
use feishu::client::FeishuClient;

#[tokio::main]
async fn main() -> anyhow::Result<()>{
    // 加载各项配置
    let config = Config::load(Path::new("config.toml"))?;
    println!("配置加载成功");
    
    // 创建客户端
    let mut feishu_client = FeishuClient::new(config.feishu)?;

    // 发送消息
    let sent_message = feishu_client.send_text(&config.bot.target_chat_id, "test").await?;
    println!("消息发送成功,消息ID: {}", sent_message.message_id);
    let sent_message = feishu_client.send_text(&config.bot.target_chat_id, "test").await?;
    println!("消息发送成功,消息ID: {}", sent_message.message_id);
    
    Ok(())
}