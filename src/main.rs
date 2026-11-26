mod app;
mod command;
mod config;
mod feishu;
mod queue;

use config::Config;
use std::path::Path;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载各项配置
    let config = Config::load(Path::new("config.toml"))?;
    println!("配置加载成功");

    // 创建停止开关
    let shutdown = CancellationToken::new();

    // 运行业务函数
    let relay = app::run(&config, &shutdown);
    let events = feishu::event::listen(&config.feishu, &shutdown, &config.bot);

    tokio::pin!(relay);
    tokio::pin!(events);

    let result = tokio::select! {
        relay_result = &mut relay => {
            if let Err(error) = &relay_result {
                eprintln!("RabbitMQ转发异常: {error}");
                let notice_result = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    notify_failure(&config, "RabbitMQ转发"),
                ).await;
                
                match notice_result {
                    Ok(Err(error)) => {
                        eprintln!("故障通知发送失败: {error}");
                    }
                    Err(error) => {
                        eprintln!("故障通知发送超时: {error}");
                    }
                    _ => {}
                }
            }

            shutdown.cancel();
            let events_result = events.await;
            
            relay_result?;
            events_result?;
            Ok(())
        }
        events_result = &mut events => {
            if let Err(error) = &events_result {
                eprintln!("飞书指令监听异常: {error}");
                let notice_result = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    notify_failure(&config, "飞书指令监听"),
                ).await;
                
                match notice_result {
                    Ok(Err(error)) => {
                        eprintln!("故障通知发送失败: {error}");
                    }
                    Err(error) => {
                        eprintln!("故障通知发送超时: {error}");
                    }
                    _ => {}
                }
            }

            shutdown.cancel();
            let relay_result = relay.await;
            
            events_result?;
            relay_result?;
            Ok(())
        }
        signal_result = tokio::signal::ctrl_c() => {
            shutdown.cancel();
            // 等待业务正常停止
            let (relay_result, events_result) = tokio::join!(relay, events);

            signal_result?;
            relay_result?;
            events_result?;
            Ok(())
        }
    };

    result
}

async fn notify_failure(config: &Config, service_name: &str) -> anyhow::Result<()> {
    // 创建客户端并发送故障通知
    let mut feishu_client_temp = crate::feishu::client::FeishuClient::new(config.feishu.clone())?;
    let notice = format!("{service_name}异常结束，机器人即将下线，请检查运行日志");

    feishu_client_temp
        .send_text(&config.bot.target_chat_id, &notice)
        .await?;
    Ok(())
}
