use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct TokenRequest {
    pub app_id: String,
    pub app_secret: String,
}

#[derive(Deserialize)]
pub struct TokenResponse {
    pub code: i32,
    pub msg: String,
    pub tenant_access_token: Option<String>,
    pub expire: Option<u64>,
}

pub async fn fetch_access_token(
    // 获取飞书凭证
    client: &reqwest::Client,
    feishu_config: &crate::config::FeishuConfig,
) -> anyhow::Result<TokenResponse> {
    let token_request = TokenRequest {
        app_id: feishu_config.app_id.clone(),
        app_secret: feishu_config.app_secret.clone(),
    };

    let post_response = client
        .post("https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal")
        .json(&token_request)
        .send()
        .await?
        .error_for_status()?;

    let token_response = post_response.json::<TokenResponse>().await?;

    if token_response.code != 0 {
        anyhow::bail!(
            "获取飞书凭证失败，错误码{}，原因{}",
            token_response.code,
            token_response.msg
        );
    }

    // 检查凭证
    match token_response.tenant_access_token.as_ref() {
        Some(token) => {
            if token.trim().is_empty() {
                anyhow::bail!("飞书返回的凭证为空");
            }
        }
        None => {
            anyhow::bail!("飞书响应缺少 tenant_access_token");
        }
    }

    // 检查凭证有效期
    match token_response.expire {
        Some(second) => {
            if second == 0 {
                anyhow::bail!("飞书返回的凭证有效期为零");
            }
        }
        None => {
            anyhow::bail!("飞书响应缺少 expire");
        }
    }

    Ok(token_response)
}
