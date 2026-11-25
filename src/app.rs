use crate::config::Config;
use crate::feishu::client::FeishuClient;
use crate::queue;
use crate::queue::is_reconnectable_consumer_error;
use tokio_util::sync::CancellationToken;

// 业务运行函数
pub async fn run(config: Config, shutdown: &CancellationToken) -> anyhow::Result<()> {
    // 创建客户端
    let mut feishu_client = FeishuClient::new(config.feishu)?;

    loop {
        // 获取 RabbitMQ 连接与通道
        let (connection, channel) =
            match queue::connect_with_retry(&config.rabbitmq, shutdown).await? {
                Some(pair) => pair,
                None => {
                    println!("连接被用户正常取消");
                    return Ok(());
                }
            };

        // 进行消费
        let consume_result = match queue::create_consumer(&channel, &config.rabbitmq.queue).await {
            Ok(consumer) => {
                println!("开始等待消息队列");
                let consume_result = queue::consume(
                    consumer,
                    &mut feishu_client,
                    &config.bot.target_chat_id,
                    shutdown,
                )
                .await; // 保存消费结果
                consume_result
            }
            Err(error) => {
                if is_reconnectable_consumer_error(&error) {
                    Ok(queue::ConsumeExit::Reconnect)
                } else {
                    Err(error.into())
                }
            }
        };

        // 保存退出结果
        let channel_close_result = channel.close(200, "shutdown".into()).await;
        let connection_close_result = connection.close(200, "shutdown".into()).await;

        // 检验关闭通道与连接关闭结果
        if let Err(error) = &channel_close_result {
            eprintln!("关闭RabbitMQ通道失败: {error}");
        }
        if let Err(error) = &connection_close_result {
            eprintln!("关闭RabbitMQ连接失败: {error}");
        }

        // 检验停机结果
        match consume_result {
            Ok(queue::ConsumeExit::Stopped) => {
                // 正常停机
                channel_close_result?;
                connection_close_result?;
                println!("feishu_bot正常停机");
                return Ok(());
            }
            Ok(queue::ConsumeExit::Reconnect) => {
                // 值得重连的非正常停机
                println!("即将在三秒后尝试重新连接");
                // 等待三秒的同时继续监听关闭信号
                tokio::select! {
                    biased;

                    _ = shutdown.cancelled() => {
                        return Ok(());
                    }

                    _ = tokio::time::sleep(
                        std::time::Duration::from_secs(3)
                    ) => {
                        println!("三秒等待结束");
                    }
                }
            }
            Err(error) => {
                // 致命错误
                return Err(error);
            }
        }
    }
}