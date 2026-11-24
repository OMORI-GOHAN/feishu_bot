use crate::feishu::auth::fetch_access_token;

pub struct FeishuClient {
    http: reqwest::Client,
    config: crate::config::FeishuConfig,
    cached_token: Option<CachedToken>,
}

struct CachedToken {
    value: String,
    refresh_at: std::time::Instant,
}

impl FeishuClient {
    // 构建客户端，实例化客户端对象
    pub fn new(config: crate::config::FeishuConfig) -> anyhow::Result<Self> {
        let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
        
        Ok(Self{
            http: http_client,
            config: config,
            cached_token: None
        })
    }

    // 查找凭据缓存
    fn cached_access_token(&self) -> Option<&str> {
        match self.cached_token.as_ref() {
            None => None,
            Some(cached_token) => {
                if std::time::Instant::now() < cached_token.refresh_at {
                    Some(cached_token.value.as_str())
                }else {
                    None
                }
            }
        }
    }

    // 刷新凭据缓存
    async fn refresh_access_token(&mut self) -> anyhow::Result<()> {
        // 发送请求并取得响应
        println!("正在获取新凭证");
        let mut refresh_at = std::time::Instant::now();
        let token_response = fetch_access_token(&self.http, &self.config).await?;

        // 计算更新时间
        let expire = token_response
            .expire
            .ok_or_else(|| anyhow::anyhow!("飞书响应缺少 expire"))?;
        refresh_at += std::time::Duration::from_secs(expire / 2); // 刷新缓存的时间为到期时间的一半

        // 获取最新凭据
        let value = token_response
            .tenant_access_token
            .ok_or_else(||anyhow::anyhow!("刷新凭据缓存，返回的凭据为空"))?;

        self.cached_token = Some(CachedToken{
            value: value,
            refresh_at: refresh_at 
        });

        Ok(())
    }

    // 获取凭据(组织缓存的查找和刷新)
    async fn get_access_token(&mut self) -> anyhow::Result<String> {
        match self.cached_access_token() {
            Some(token) => {
                println!("凭证缓存命中");
                Ok(token.to_string())
            },
            None => {
                self.refresh_access_token().await?;
                Ok(self.cached_access_token()
                    .ok_or_else(|| anyhow::anyhow!("凭据刷新后仍不可用"))?
                    .to_string())
            }
        }
    }

    // 安排消息发送
    pub async fn send_text(
        &mut self,
        chat_id: &str,
        text: &str,
    )   -> anyhow:: Result<crate::feishu::message::SentMessage> {
        let access_token = self.get_access_token().await?;
        Ok(crate::feishu::message::send_text(
            &self.http,
            &access_token,
            chat_id,
            text
        ).await?)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::FeishuConfig;
    use super::*;

    #[test]
    fn expire_token_is_not_reused() {
        // 创建测试客户端
        let feishu_config_test = FeishuConfig{
            app_id: "test-app".to_string(), 
            app_secret: "test-secret".to_string()
        };
        let mut client_test = FeishuClient::new(feishu_config_test).expect("创建测试客户端失败");
        client_test.cached_token = Some(CachedToken { 
            value: "test".to_string(), 
            refresh_at: std::time::Instant::now() 
        });

        let result = client_test.cached_access_token();
        assert!(result.is_none());
    }
}