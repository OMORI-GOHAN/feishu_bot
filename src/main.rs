mod app;
mod command;
mod config;
mod feishu;
mod queue;

use config::Config;
use std::path::Path;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载各项配置
    let config = Config::load(Path::new("config.toml"))?;
    println!("配置加载成功");

    // 创建停止开关
    let shutdown = CancellationToken::new();

    // 运行业务函数
    // let running = app::run(config, &shutdown);
    let running = feishu::event::listen(&config.feishu, &shutdown, &config.bot);
    tokio::pin!(running);

    let result = tokio::select! {
        running_result = &mut running => {
            running_result   // 若业务异常停止，那么在这里返回错误
        }

        signal_result = tokio::signal::ctrl_c() => {
            shutdown.cancel();
            // 等待业务正常停止
            let finish_result = running.await;
            signal_result?;
            finish_result
        }
    };

    result
}
