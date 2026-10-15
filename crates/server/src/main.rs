use packy_common::models::Credentials;
use packy_server::{
    api,
    application::{AppState, auth},
    infrastructure::{config::Settings, database::Database},
};
use std::io::{self, BufRead};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::load()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_new(&settings.log_filter)
                .map_err(|_| anyhow::anyhow!("RUST_LOG / log_filter 格式无效"))?,
        )
        .init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "init-admin") {
        if args.len() != 2 {
            anyhow::bail!("用法：packy-server init-admin <账号>，密码从标准输入读取");
        }
        let mut password = String::new();
        io::stdin().lock().read_line(&mut password)?;
        // 标准输入的换行不属于密码，其他空白字节保持原样
        let password = password.trim_end_matches(['\r', '\n']).to_owned();
        let db = Database::open(&settings.database).await?;
        let created = auth::bootstrap(
            &db,
            Credentials {
                username: args[1].clone(),
                password,
            },
        )
        .await?;
        println!(
            "{}",
            if created {
                "超管初始化完成"
            } else {
                "已存在超管，未修改任何账号"
            }
        );
        return Ok(());
    }
    if !args.is_empty() && args != ["serve"] {
        anyhow::bail!("可用命令：serve、init-admin <账号>");
    }
    let config = settings.into_config()?;
    let listen = config.listen;
    let state = AppState::open(config).await?;
    let listener = tokio::net::TcpListener::bind(listen).await?;
    tracing::info!(%listen, "HTTP 服务已启动");
    axum::serve(listener, api::router(state))
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

/// 同时响应 Ctrl+C 和服务管理器的终止信号，等待进行中的请求退出
async fn shutdown() {
    #[cfg(unix)]
    {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! { result = tokio::signal::ctrl_c() => {
                    if let Err(error) = result { tracing::error!(%error, "无法监听中断信号"); }
                }, _ = terminate.recv() => {} }
            }
            Err(error) => tracing::error!(%error, "无法监听终止信号"),
        }
    }
    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "无法监听中断信号");
    }
}
