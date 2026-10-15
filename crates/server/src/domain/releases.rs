use packy_common::{
    error::{Error, Result},
    models::*,
    validation,
};
use std::collections::{BTreeMap, HashSet};

/// 完整快照需要唯一的稳定 Mod 身份和目标路径，不因名称重复合并文件
pub fn validate_snapshot(items: &[ReleaseItem]) -> Result<()> {
    if items.len() > 2000 {
        return Err(Error::invalid("一个整合包最多管理 2000 个 Mod"));
    }
    let mut ids = HashSet::new();
    let mut paths = HashSet::new();
    for item in items {
        if !ids.insert(&item.mod_id) || !paths.insert(item.target.path.to_lowercase()) {
            return Err(Error::invalid("Mod 身份或目标路径重复"));
        }
        validation::mod_path(&item.target.path)?;
        validation::sha512(&item.target.sha512)?;
        if item.target.sources.is_empty() || item.target.size.is_some_and(|size| size < 0) {
            return Err(Error::invalid("文件缺少下载来源或大小非法"));
        }
        for source in &item.target.sources {
            validation::http_url(&source.url)?;
            if source.priority < 0 {
                return Err(Error::invalid("来源排序不能为负数"));
            }
        }
    }
    validation::non_overlapping_paths(items.iter().map(|item| item.target.path.as_str()))?;
    Ok(())
}

/// 只比较两个不可变清单的路径和哈希，路径复用交由客户端先删旧路径、再安装全部目标
pub fn difference(old: &[ReleaseItem], new: &[ReleaseItem]) -> Result<Vec<Operation>> {
    validate_snapshot(old)?;
    validate_snapshot(new)?;
    validation::non_overlapping_paths(old.iter().chain(new).map(|item| item.target.path.as_str()))?;
    let old: BTreeMap<_, _> = old
        .iter()
        .map(|item| (item.mod_id.as_str(), item))
        .collect();
    let new: BTreeMap<_, _> = new
        .iter()
        .map(|item| (item.mod_id.as_str(), item))
        .collect();
    let mut operations = Vec::new();
    for (key, previous) in &old {
        if !new.contains_key(key) {
            operations.push(Operation {
                action: Action::Remove,
                mod_id: (*key).into(),
                old_path: Some(previous.target.path.clone()),
                target: None,
            });
        }
    }
    for (key, target) in &new {
        match old.get(key) {
            None => operations.push(Operation {
                action: Action::Add,
                mod_id: (*key).into(),
                old_path: None,
                target: Some(target.target.clone()),
            }),
            Some(previous)
                if previous.target.path != target.target.path
                    || previous.target.sha512 != target.target.sha512 =>
            {
                operations.push(Operation {
                    action: Action::Replace,
                    mod_id: (*key).into(),
                    old_path: Some(previous.target.path.clone()),
                    target: Some(target.target.clone()),
                })
            }
            _ => {}
        }
    }
    Ok(operations)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item(id: &str, path: &str, hash: char) -> ReleaseItem {
        ReleaseItem {
            mod_id: id.into(),
            artifact_id: format!("artifact-{id}"),
            modid_snapshot: "same-name".into(),
            name_snapshot: "重复名称".into(),
            target: TargetFile {
                path: path.into(),
                size: None,
                sha512: hash.to_string().repeat(128),
                sources: vec![DownloadSource {
                    r#type: SourceType::Server,
                    url: "http://127.0.0.1/file.jar".into(),
                    priority: 0,
                }],
            },
        }
    }
    #[test]
    fn skipped_versions_produce_final_changes_without_names_as_identity()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let old = vec![
            item("a", "mods/old.jar", 'a'),
            item("b", "mods/delete.jar", 'b'),
        ];
        let new = vec![
            item("a", "mods/new.jar", 'c'),
            item("c", "mods/add.jar", 'd'),
        ];
        let operations = difference(&old, &new)?;
        assert_eq!(operations.len(), 3);
        assert_eq!(operations[0].action, Action::Remove);
        assert_eq!(operations[1].action, Action::Replace);
        assert_eq!(operations[2].action, Action::Add);
        assert!(difference(&new, &new)?.is_empty());
        let collision = vec![item("a", "mods/A.jar", 'a'), item("b", "mods/a.jar", 'b')];
        assert!(validate_snapshot(&collision).is_err());
        Ok(())
    }
}
