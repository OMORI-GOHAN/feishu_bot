use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct SendMessageRequest {
    pub receive_id: String, // 接收者 ID
    pub msg_type: String,   // 消息类型
    pub content: String,    // 消息内容的 JSON 字符串
}

#[derive(Deserialize)]
pub struct SendMessageResponse {
    pub code: i32,                    // 飞书业务结果码
    pub msg: String,                  // 本次接口调用的结果说明
    pub data: Option<MessageReceipt>, // 返回的消息数据；
}

#[derive(Deserialize)]
pub struct MessageReceipt {
    pub message_id: String, // 飞书为本次发送生成的聊天消息 ID
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text { text: String },
    Image { image_key: String },
}

// 收到的文字消息
pub struct IncomingMessage {
    pub message_id: String,                // 原消息id
    pub chat_id: String,                   // 所在群id
    pub sender_open_id: String,            // 发送者id
    pub text: String,                      // 正文
    pub parent_message_id: Option<String>, // 被回复时引用的消息的id
}

// 构造请求
pub fn build_text_request(chat_id: &str, text: &str) -> anyhow::Result<SendMessageRequest> {
    if chat_id.trim().is_empty() {
        anyhow::bail!("群id为空!");
    }

    let send_str = if text.trim().is_empty() {
        "[反馈正文为空]"
    } else {
        text
    };

    let text_content = MessageContent::Text {
        text: send_str.to_string(),
    };

    Ok(SendMessageRequest {
        receive_id: chat_id.to_string(),
        msg_type: "text".to_string(),
        content: serde_json::to_string(&text_content)?,
    })
}

// 发送消息
pub async fn send_text(
    client: &reqwest::Client,
    access_token: &str,
    chat_id: &str,
    text: &str,
) -> anyhow::Result<MessageReceipt> {
    let text_request = build_text_request(chat_id, text)?;

    // 发送
    let post_response = client
        .post("https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type=chat_id")
        .bearer_auth(access_token)
        .json(&text_request)
        .send()
        .await?
        .error_for_status()?;

    // 解析响应
    let send_message_response = post_response.json::<SendMessageResponse>().await?;

    if send_message_response.code != 0 {
        anyhow::bail!(
            "消息发送失败，错误码{}，原因{}",
            send_message_response.code,
            send_message_response.msg
        );
    }

    // 从响应中取出消息回执
    let message_receipt = match send_message_response.data {
        Some(receipt) => {
            if receipt.message_id.trim().is_empty() {
                anyhow::bail!("响应id为空");
            }
            receipt
        }
        None => {
            anyhow::bail!("响应缺少data");
        }
    };

    Ok(message_receipt)
}

pub async fn reply_message(
    client: &reqwest::Client,
    access_token: &str,
    message_id: &str,
    content: &MessageContent,
) -> anyhow::Result<MessageReceipt> {
    if message_id.trim().is_empty() {
        anyhow::bail!("回复消息时目标消息id为空");
    }
    // 判断消息类型
    let msg_type = match content {
        MessageContent::Image { .. } => "image",
        MessageContent::Text { .. } => "text",
    };

    // 序列化MessageContent
    let content_json = serde_json::to_string(content)?;

    // 序列化正文
    let text_request = serde_json::json!({
        "msg_type": msg_type,
        "content": content_json,
    });

    // 构造地址
    let url = format!("https://open.feishu.cn/open-apis/im/v1/messages/{message_id}/reply");

    // 发送消息
    let post_response = client
        .post(url)
        .bearer_auth(access_token)
        .json(&text_request)
        .send()
        .await?
        .error_for_status()?;

    // 解析响应
    let send_message_response = post_response.json::<SendMessageResponse>().await?;

    if send_message_response.code != 0 {
        anyhow::bail!(
            "回复消息发送失败，错误码{}，原因{}",
            send_message_response.code,
            send_message_response.msg
        );
    }

    // 从响应中取出消息回执
    let message_receipt = match send_message_response.data {
        Some(receipt) => {
            if receipt.message_id.trim().is_empty() {
                anyhow::bail!("响应id为空");
            }
            receipt
        }
        None => {
            anyhow::bail!("响应缺少data");
        }
    };

    Ok(message_receipt)
}
