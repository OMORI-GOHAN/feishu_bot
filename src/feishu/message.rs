use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct TextContent {
    pub text: String,
}

#[derive(Serialize)]
pub struct SendMessageRequest {
    pub receive_id: String,
    pub msg_type: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct SendMessageResponse {
    pub code: i32,
    pub msg: String,
    pub data: Option<SentMessage>,
}

#[derive(Deserialize)]
pub struct SentMessage {
    pub message_id: String,
}

// 构造请求
pub fn build_text_request(
    chat_id: &str,
    text: &str,
) -> anyhow::Result<SendMessageRequest> {
    if chat_id.trim().is_empty() {
        anyhow::bail!("群id为空!");
    }
 
    let send_str = if text.trim().is_empty() {
        "[反馈正文为空]"
    } else {
        text
    };

    let text_content = TextContent {
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
) -> anyhow::Result<SentMessage> {
    let text_request = build_text_request(chat_id, text)?;

    // 发送
    let post_response = client.post("https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type=chat_id")
        .bearer_auth(access_token)
        .json(&text_request)
        .send().await?
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
    
    // 取出结构体SentMessageResponse中的SentMessage
    let sent_message = match send_message_response.data {
        Some(sent_message) => {
            if sent_message.message_id.trim().is_empty() {
                anyhow::bail!("响应id为空");
            }
            sent_message
        },
        None => {
            anyhow::bail!("响应缺少data");
        }
    };

    Ok(sent_message)
}