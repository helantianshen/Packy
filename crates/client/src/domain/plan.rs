use crate::error::{Error, Result};
use packy_common::{error::Error as RuleError, models::*, validation};
use std::collections::HashSet;

/// 服务端计划仍是不可信输入，全部校验通过后才允许建立本地任务
pub fn validate(plan: &UpdatePlan, pack_id: &str) -> Result<()> {
    if plan.pack_id != pack_id
        || uuid::Uuid::parse_str(&plan.target_release_id).is_err()
        || plan.operations.len() > 4000
    {
        return Err(RuleError::invalid("更新计划的整合包、目标版本或操作数量不正确").into());
    }
    let mut mods = HashSet::new();
    let mut old_paths = HashSet::new();
    let mut targets = HashSet::new();
    for operation in &plan.operations {
        if uuid::Uuid::parse_str(&operation.mod_id).is_err() || !mods.insert(&operation.mod_id) {
            return Err(RuleError::invalid("更新计划包含非法或重复 Mod 身份").into());
        }
        match (operation.action, &operation.old_path, &operation.target) {
            (Action::Add, None, Some(_))
            | (Action::Replace, Some(_), Some(_))
            | (Action::Remove, Some(_), None) => {}
            _ => {
                return Err(
                    RuleError::invalid("更新操作字段不符合 ADD/REPLACE/REMOVE 规则").into(),
                );
            }
        }
        if let Some(path) = &operation.old_path {
            validation::mod_path(path)?;
            if !old_paths.insert(path.to_lowercase()) {
                return Err(RuleError::invalid("更新计划重复删除同一路径").into());
            }
        }
        if let Some(target) = &operation.target {
            validation::mod_path(&target.path)?;
            validation::sha512(&target.sha512)?;
            if !targets.insert(target.path.to_lowercase())
                || target.size.is_some_and(|n| n < 0)
                || target.sources.is_empty()
                || target.sources.len() > 16
            {
                return Err(RuleError::invalid("更新目标路径、大小或来源不正确").into());
            }
            for source in &target.sources {
                validation::http_url(&source.url)?;
                if source.priority < 0 {
                    return Err(RuleError::invalid("来源优先级不能为负数").into());
                }
            }
        }
    }
    validation::non_overlapping_paths(plan.operations.iter().flat_map(|op| {
        op.old_path
            .iter()
            .map(String::as_str)
            .chain(op.target.iter().map(|target| target.path.as_str()))
    }))?;
    Ok(())
}

/// 取消只作用于下载和校验，文件安装和版本保存必须完整结束
pub fn cancellable(phase: &str) -> bool {
    matches!(phase, "checking" | "downloading" | "verifying")
}
/// 这两个阶段的退出会破坏文件和版本一致性，托盘退出也必须检查
pub fn installing(phase: &str) -> bool {
    matches!(phase, "installing" | "saving_state")
}
/// 从原记录恢复时不查询服务器最新发布，版本差异仅允许原版本或已经落盘的目标版本
pub fn validate_record(
    record: &TaskRecord,
    config: &LocalConfig,
    state: &LocalState,
) -> Result<()> {
    if !matches!(
        record.phase.as_str(),
        "downloading" | "verifying" | "installing" | "saving_state" | "completed"
    ) || (record.phase == "completed"
        && state.installed_release_id != record.plan.target_release_id)
        || record.schema_version != 1
        || uuid::Uuid::parse_str(&record.task_id).is_err()
        || (record.installed_release_id != state.installed_release_id
            && record.plan.target_release_id != state.installed_release_id)
    {
        return Err(Error::conflict(
            "本地任务与已安装版本不一致，请恢复整合包记录",
        ));
    }
    validate(&record.plan, &config.pack_id)
}
