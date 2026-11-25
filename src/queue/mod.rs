use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

// 建立与RabbitMQ的连接
pub async fn connect(
    config: &crate::config::RabbitMqConfig,
) -> lapin::Result<(lapin::Connection, lapin::Channel)> {
    // 连接RabbitMQ
    let connection =
        lapin::Connection::connect(&config.url, lapin::ConnectionProperties::default()).await?;

    // 创建通道
    let channel = connection.create_channel().await?;

    Ok((connection, channel))
}

// 创建消费者
pub async fn create_consumer(
    channel: &lapin::Channel,
    queue_name: &str,
) -> anyhow::Result<lapin::Consumer> {
    // 限制未确认消息数量
    channel
        .basic_qos(
            1, // 最多一条未处理消息
            lapin::options::BasicQosOptions::default(),
        )
        .await?;

    let consumer = channel
        .basic_consume(
            queue_name.into(),                              // 队列名
            "feishu_bot_relay".into(),                      // 消费者标识
            lapin::options::BasicConsumeOptions::default(), // 消费选项
            lapin::types::FieldTable::default(),            // 额外参数
        )
        .await?;

    Ok(consumer)
}

// 消费流
pub async fn consume(
    mut consumer: lapin::Consumer,
    feishu_client: &mut crate::feishu::client::FeishuClient,
    chat_id: &str,
    shutdown: &CancellationToken,
) -> anyhow::Result<()> {
    // 设置退出事件监听

    loop {
        tokio::select! {
            biased;

            _ = shutdown.cancelled() => {
                println!("收到停止信号");
                return Ok(());
            }

            item = consumer.next() => {
                match item {
                    Some(result) => {
                        // 提取消息deliver，在此我们需要其中的 data正文，并使用ack()确认投递完成
                        let delivery = result?;
                        let text = std::str::from_utf8(&delivery.data)?;
                        feishu_client.send_text(chat_id, text).await?;
                        delivery
                            .ack(lapin::options::BasicAckOptions::default())
                            .await?;
                    },
                    None => {
                        anyhow::bail!("RabbitMQ消费流意外退出!")
                    }
                }
            }
        }
    }
}

// 允许重连的连接尝试
pub async fn connect_with_retry(
    config: &crate::config::RabbitMqConfig,
    shutdown: &CancellationToken,
) -> anyhow::Result<Option<(lapin::Connection, lapin::Channel)>> {
    loop {
        let connect_result = tokio::select! {
            biased;
            _ = shutdown.cancelled() => {
                println!("收到停止信号");
                return Ok(None);
            }

            result = connect(config) => { // 连接并保存当前连接尝试的结果
                result
            }
        };

        // 检查连接情况
        match connect_result {
            Ok(pair) => {
                println!("连接成功");
                return Ok(Some(pair));
            }
            Err(error) => {
                if is_retryable_connection_error(&error) {
                    println!("连接错误，三秒后尝试重连"); // 这里有点难懂，是判断可以继续连接后，继续往下走↓
                } else {
                    println!("致命错误，无法重连");
                    return Err(error.into()); // 得到无法重连的错误后直接返回，不继续执行下方的内容
                }
            }
        }

        // 等待三秒的同时继续监听关闭信号
        tokio::select! {
            biased;

            _ = shutdown.cancelled() => {
                return Ok(None);
            }

            _ = tokio::time::sleep(
                std::time::Duration::from_secs(3)
            ) => {
                println!("三秒等待结束");
            }
        }
    }
}

// 判断是否为值得重试的连接错误
fn is_retryable_connection_error(error: &lapin::Error) -> bool {
    match error.kind() {
        lapin::ErrorKind::IOError(io_error) => match io_error.kind() {
            std::io::ErrorKind::ConnectionRefused
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::Interrupted => true,
            _ => false,
        },
        _ => false,
    }
}
