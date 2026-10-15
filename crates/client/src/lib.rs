//! 与桌面框架无关的更新核心，负责本地任务和文件操作的最终判定
pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;

use packy_common::models::ProbeInfo;
/// 返回核心版本供桌面检查 IPC，不能作为业务更新成功的依据
pub fn probe() -> ProbeInfo {
    ProbeInfo {
        name: "Packy".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        phase: "P4".into(),
        business_enabled: true,
        checked_at: String::new(),
    }
}
