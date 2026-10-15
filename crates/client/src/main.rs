use packy_client::application::Client;
use std::path::PathBuf;

/// 无界面入口用于部署联调，桌面和命令行复用相同核心而不复制安装逻辑
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        anyhow::bail!("用法：packy-client <整合包目录> <check|update|retry|status|launch>");
    }
    let client = Client::open(&PathBuf::from(&args[0])).await?;
    match args[1].as_str() {
        "check" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&client.check_update().await?)?
            );
        }
        "status" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&client.snapshot().await)?
            );
        }
        "launch" => client.launch_game().await?,
        "update" | "retry" => {
            let mut events = client.subscribe();
            if args[1] == "retry" {
                let snapshot = client.snapshot().await;
                client.retry_update(&snapshot.status.task_id).await?;
            } else {
                let plan = client.check_update().await?;
                if plan.operations.is_empty() {
                    println!("当前已是最新版本");
                    return Ok(());
                }
                client.start_update().await?;
            }
            loop {
                if !client.running().await {
                    let snapshot = client.snapshot().await;
                    println!("{}", serde_json::to_string_pretty(&snapshot)?);
                    if snapshot.status.phase != "completed" {
                        anyhow::bail!("更新未完成");
                    }
                    break;
                }
                match events.recv().await {
                    Ok(status) => eprintln!(
                        "{} {}/{}",
                        status.phase, status.completed_files, status.total_files
                    ),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(error) => return Err(error.into()),
                }
            }
        }
        _ => anyhow::bail!("未知命令"),
    }
    Ok(())
}
