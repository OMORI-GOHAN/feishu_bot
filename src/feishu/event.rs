use feishu_ws::{Event, Handler};

use crate::feishu::message::MessageContent;

// 事件处理器
struct EventPrinter {
    // 目标群id
    target_chat_id: String,
    // 指令消息发送端
    message_tx: tokio::sync::mpsc::Sender<IncomingMessage>,
}

// 收到的文字消息
pub struct IncomingMessage {
    // 原消息id
    pub message_id: String,
    // 所在群id
    pub chat_id: String,
    // 发送者id
    pub sender_open_id: String,
    // 正文
    pub text: String,
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
                    "message_id: {0}, chat_id: {1}, sender_open_id: {2}, text: {3}",
                    incoming_message.message_id,
                    incoming_message.chat_id,
                    incoming_message.sender_open_id,
                    incoming_message.text
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

// 依次接收消息并执行指令
async fn process_commands(
    mut message_rx: tokio::sync::mpsc::Receiver<IncomingMessage>,
    feishu_client: &mut crate::feishu::client::FeishuClient,
    shutdown: &tokio_util::sync::CancellationToken,
) -> anyhow::Result<()> {
    loop {
        tokio::select! {
            // 同时就绪时优先停止
            biased;

            _ = shutdown.cancelled() => {
                return Ok(());
            }
            item = message_rx.recv() => {
                match item {
                    Some(incoming_message) => {
                        if let Some(command) = crate::command::parse_command(&incoming_message.text) {
                            if let Err(error) = crate::command::execute_command(feishu_client, &incoming_message.message_id, &command).await {
                                eprintln!("错误: {error}");
                            }
                        }
                    },
                    None => {
                        anyhow::bail!("内部消息通道已关闭");
                    }
                }
            }
        }
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
    let (message_tx, message_rx) = tokio::sync::mpsc::channel::<IncomingMessage>(32);

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

    // 跟随主程序停止，也可单独通知两项工作收尾
    let listener_shutdown = shutdown.child_token();

    // 创建两个Future，由下方select同时推动执行
    let running = ws_client.run(&handler, listener_shutdown.cancelled());
    let commands = process_commands(message_rx, &mut feishu_client, &listener_shutdown);

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

// 提取文字消息，忽略其他事件与消息类型
fn parse_message(event: &Event) -> anyhow::Result<Option<IncomingMessage>> {
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

                    // 将正文JSON解析为文字内容
                    let message_content: crate::feishu::message::MessageContent =
                        serde_json::from_str(content)?;
                    let text = match message_content {
                        MessageContent::Text { text } => text,
                        MessageContent::Image { .. } => {
                            anyhow::bail!("消息正文识别为文字，却试图输出图片");
                        }
                    };
                    let incoming_message = IncomingMessage {
                        message_id: message_id.to_owned(),
                        chat_id: chat_id.to_owned(),
                        sender_open_id: sender_open_id.to_owned(),
                        text: text,
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
