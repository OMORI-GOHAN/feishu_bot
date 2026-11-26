use feishu_ws::{Event, Handler};

use crate::feishu::message::MessageContent;

// 事件处理器
struct EventPrinter {
    // 目标群id
    target_chat_id: String,
    // 指令消息发送端
    message_tx: tokio::sync::mpsc::Sender<super::message::IncomingMessage>,
}

// 处理事件与连接状态
impl Handler for EventPrinter {
    fn on_event(&self, event: Event) {
        match parse_message(&event) {
            Ok(Some(incoming_message)) => {
                // 忽略其他群的消息
                if incoming_message.chat_id != self.target_chat_id {
                    return;
                }
                println!(
                    "message_id: {0}, chat_id: {1}, sender_open_id: {2}, text: {3}, parent_message_id: {4:?}",
                    incoming_message.message_id,
                    incoming_message.chat_id,
                    incoming_message.sender_open_id,
                    incoming_message.text,
                    incoming_message.parent_message_id,
                );
                // 转交消息，通道满或关闭时丢弃并记录错误
                if let Err(error) = self.message_tx.try_send(incoming_message) {
                    eprintln!("消息转交指令处理失败： {error}");
                }
            }
            Ok(None) => {}
            Err(error) => {
                eprintln!("出现错误: {:?}", error);
            }
        }
    }

    fn on_state(&self, state: feishu_ws::State) {
        println!("{:?}", state);
    }
}

// 同时运行长连接与指令处理
pub async fn listen(
    feishu_config: &crate::config::FeishuConfig,
    shutdown: &tokio_util::sync::CancellationToken,
    bot_config: &crate::config::BotConfig,
) -> anyhow::Result<()> {
    // 创建长连接客户端
    let ws_config = feishu_ws::Config::new(
        feishu_config.app_id.clone(),
        feishu_config.app_secret.clone(),
    );
    let ws_client = feishu_ws::Client::new(ws_config)?;

    // 用内部通道转交消息，最多暂存32条
    let (message_tx, message_rx) =
        tokio::sync::mpsc::channel::<super::message::IncomingMessage>(32);

    // 创建处理器
    let handler = EventPrinter {
        target_chat_id: bot_config.target_chat_id.clone(),
        message_tx,
    };

    // 复用回复客户端的HTTP连接和凭据缓存
    let mut feishu_client =
        crate::feishu::client::FeishuClient::new(crate::config::FeishuConfig {
            app_id: feishu_config.app_id.clone(),
            app_secret: feishu_config.app_secret.clone(),
        })?;

    let listener_shutdown = shutdown.child_token();

    // 创建两个Future，由下方select同时推动执行
    let running = ws_client.run(&handler, listener_shutdown.cancelled());
    let commands =
        crate::command::process_commands(message_rx, &mut feishu_client, &listener_shutdown);

    tokio::pin!(running);
    tokio::pin!(commands);

    tokio::select! {
        running_result = &mut running => {
            // 长连接结束，通知指令处理停止并等待当前指令完成
            listener_shutdown.cancel();
            let command_result = commands.await;

            // 优先返回长连接的错误
            running_result?;
            command_result?;

            return Ok(());
        }
        command_result = &mut commands => {
            // 指令处理结束，通知长连接停止并等待收尾
            listener_shutdown.cancel();
            let running_result = running.await;

            // 优先返回指令处理的错误
            command_result?;
            running_result?;

            return Ok(());
        }
    }
}

// 提取收到的文字消息
fn parse_message(event: &Event) -> anyhow::Result<Option<super::message::IncomingMessage>> {
    match event.event_type() {
        Some("im.message.receive_v1") => {
            match event
                .payload
                .pointer("/event/message/message_type")
                .and_then(|value| value.as_str())
            {
                Some("text") => {
                    // 提取消息标识、发送者与正文
                    let message_id = event
                        .payload
                        .pointer("/event/message/message_id")
                        .and_then(|value| value.as_str())
                        .ok_or_else(|| anyhow::anyhow!("消息事件缺少字段 message_id"))?;

                    let chat_id = event
                        .payload
                        .pointer("/event/message/chat_id")
                        .and_then(|value| value.as_str())
                        .ok_or_else(|| anyhow::anyhow!("消息事件缺少字段 chat_id"))?;

                    let sender_open_id = event
                        .payload
                        .pointer("/event/sender/sender_id/open_id")
                        .and_then(|value| value.as_str())
                        .ok_or_else(|| anyhow::anyhow!("消息事件缺少字段 sender_id/open_id"))?;

                    let content = event
                        .payload
                        .pointer("/event/message/content")
                        .and_then(|value| value.as_str())
                        .ok_or_else(|| anyhow::anyhow!("消息事件缺少字段 content"))?;

                    let parent_message_id = event
                        .payload
                        .pointer("/event/message/parent_id")
                        .and_then(|value| value.as_str())
                        .map(str::to_owned);

                    // 将正文JSON解析为文字内容
                    let message_content: crate::feishu::message::MessageContent =
                        serde_json::from_str(content)?;
                    let text = match message_content {
                        MessageContent::Text { text } => text,
                        MessageContent::Image { .. } => {
                            anyhow::bail!("消息正文识别为文字，却试图输出图片");
                        }
                    };
                    let incoming_message = super::message::IncomingMessage {
                        message_id: message_id.to_owned(),
                        chat_id: chat_id.to_owned(),
                        sender_open_id: sender_open_id.to_owned(),
                        text: text,
                        parent_message_id,
                    };
                    Ok(Some(incoming_message))
                }
                Some(_) => Ok(None),
                None => {
                    anyhow::bail!("缺少字符串字段 message_type")
                }
            }
        }
        _ => Ok(None),
    }
}
