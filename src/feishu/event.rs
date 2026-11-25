use feishu_ws::{Event, Handler};

// 事件处理器
struct EventPrinter {
    target_chat_id: String,
}

pub struct IncomingMessage {
    pub message_id: String,     // 消息id
    pub chat_id: String,        // 消息群id
    pub sender_open_id: String, // 发送者id
    pub text: String,           // 正文
}

// 使处理器实现状态与事件监听
impl Handler for EventPrinter {
    fn on_event(&self, event: Event) {
        match parse_message(&event) {
            Ok(Some(incoming_message)) => {
                if incoming_message.chat_id != self.target_chat_id {
                    return; // 若目标群与实际聊天群id不符则不处理
                }
                println!(
                    "message_id: {0}, chat_id: {1}, sender_open_id: {2}, text: {3}",
                    incoming_message.message_id,
                    incoming_message.chat_id,
                    incoming_message.sender_open_id,
                    incoming_message.text
                );

                if let Some(command) = crate::command::parse_command(incoming_message.text.as_str())
                {
                    println!("接收到指令: {:?}, 参数: {:?}", command.name, command.args);
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

// 建立飞书长连接，持续接受事件
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

    // 创建处理器
    let handler = EventPrinter {
        target_chat_id: bot_config.target_chat_id.clone(),
    };

    // 运行客户端
    ws_client.run(&handler, shutdown.cancelled()).await?;

    Ok(())
}

//  解析收到的消息
fn parse_message(event: &Event) -> anyhow::Result<Option<IncomingMessage>> {
    match event.event_type() {
        Some("im.message.receive_v1") => {
            match event
                .payload
                .pointer("/event/message/message_type")
                .and_then(|value| value.as_str())
            {
                Some("text") => {
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

                    // 反序列化解析正文
                    let text: crate::feishu::message::TextContent = serde_json::from_str(content)?;

                    let incoming_message = IncomingMessage {
                        message_id: message_id.to_owned(),
                        chat_id: chat_id.to_owned(),
                        sender_open_id: sender_open_id.to_owned(),
                        text: text.text,
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
