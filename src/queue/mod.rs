use futures_util::StreamExt;

// 建立与RabbitMQ的连接
pub async fn connect(
    config: &crate::config::RabbitMqConfig,
) -> anyhow::Result<(lapin::Connection, lapin::Channel)> {
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
) -> anyhow::Result<()> {
    while let Some(result) = consumer.next().await {
        // 提取消息deliver，在此我们需要其中的 data正文，并使用ack()确认投递完成
        let delivery = result?;
        let text = std::str::from_utf8(&delivery.data)?; // 提取正文
        feishu_client.send_text(chat_id, text).await?;
        delivery
            .ack(lapin::options::BasicAckOptions::default())
            .await?; // 确认消息
    }

    anyhow::bail!("RabbitMQ消费流意外退出!")
}
