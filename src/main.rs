mod config;
mod feishu;

use config::Config;
use std::path::Path;
use crate::feishu::auth::fetch_access_token;
use crate::feishu::message::send_text;

#[tokio::main]
async fn main() -> anyhow::Result<()>{
    // 加载各项配置
    let config = Config::load(Path::new("config.toml"))?;
    println!("配置加载成功");
    
    // 创建 http 客户端
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    
    // 获取凭据
    let token_response = fetch_access_token(&http_client, &config.feishu).await?;
    let access_token = match token_response.tenant_access_token {
        Some(token) => token,
        None => {
            anyhow::bail!("响应返回的凭据为空");
        }
    };
    println!("凭据获取成功");

    // 消息发送
    let sent_message = send_text(
        &http_client,
        &access_token,
        &config.bot.target_chat_id,
        "test"
    ).await?;
    println!("消息发送成功,消息ID: {}", sent_message.message_id);
    
    Ok(())
}
