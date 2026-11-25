mod config;
mod feishu;
mod queue;

use config::Config;
use feishu::client::FeishuClient;
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
    let running = run(config, &shutdown);
    tokio::pin!(running);

    let result = tokio::select! {
        running_result = &mut running => {
            running_result   // 若业务异常停止，那么在这里返回错误
        }

        signal_result = tokio::signal::ctrl_c() => {
            shutdown.cancel();
            // 等待业务正常停止
            let finish_result = running.await;
            signal_result?;
            finish_result
        }
    };

    result
}

// 业务运行函数
async fn run(config: Config, shutdown: &CancellationToken) -> anyhow::Result<()> {
    // 创建客户端
    let mut feishu_client = FeishuClient::new(config.feishu)?;

    // 获取 RabbitMQ 连接与通道
    let (connection, channel) = match queue::connect_with_retry(&config.rabbitmq, shutdown).await? {
        Some(pair) => pair,
        None => {
            println!("连接被用户正常取消");
            return Ok(());
        }
    };

    // 创建消费者并消费消息队列
    let consumer = queue::create_consumer(&channel, &config.rabbitmq.queue).await?;
    println!("开始等待消息队列");
    let consume_result = queue::consume(
        consumer,
        &mut feishu_client,
        &config.bot.target_chat_id,
        shutdown,
    )
    .await; // 保存消费结果

    // 保存退出结果
    let channel_close_result = channel.close(200, "shutdown".into()).await;
    let connection_close_result = connection.close(200, "shutdown".into()).await;

    // 检验
    consume_result?;
    channel_close_result?;
    connection_close_result?;

    println!("feishu_bot已正常停止");
    Ok(())
}
