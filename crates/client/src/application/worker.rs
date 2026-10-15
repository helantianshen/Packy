use super::Client;
use crate::{
    error::{Error, Result},
    infrastructure::{
        download, paths,
        storage::{self, TemporaryFile},
    },
};
use futures_util::StreamExt;
use packy_common::{error::Error as RuleError, models::*};
use std::{path::PathBuf, time::Duration};

impl Client {
    /// 下载先全部落入独立任务目录，安装前不改任何原 Mod 文件
    pub(super) async fn execute(&self, record: &TaskRecord) -> Result<()> {
        self.phase("downloading").await?;
        let stage_relative = format!(".updater/staging/{}", record.task_id);
        let stage = paths::checked(&self.root, &stage_relative)?;
        tokio::fs::create_dir_all(&stage).await?;
        let targets: Vec<_> = record
            .plan
            .operations
            .iter()
            .filter_map(|op| op.target.clone())
            .collect();
        let cancel = self.inner.lock().await.cancel.clone();
        let jobs = futures_util::stream::iter(targets.clone().into_iter().enumerate()).map(
            |(index, target)| {
                let stage_relative = &stage_relative;
                let cancel = &cancel;
                async move {
                    let relative = format!("{stage_relative}/{index}.jar");
                    let path = download::file(
                        &self.http,
                        &self.root,
                        &relative,
                        &target,
                        cancel,
                        |bytes| self.bytes(index, bytes),
                    )
                    .await?;
                    self.file_finished().await;
                    Ok::<_, Error>((index, path))
                }
            },
        );
        // 首个失败取消其余下载，并等待所有暂存写入结束后才允许重试或退出
        let results = jobs
            .buffer_unordered(4)
            .inspect(|result| {
                if result.is_err() {
                    cancel.cancel();
                }
            })
            .collect::<Vec<_>>()
            .await;
        let mut staged: Vec<(usize, PathBuf)> = results.into_iter().collect::<Result<_>>()?;
        staged.sort_by_key(|(index, _)| *index);
        self.phase("verifying").await?;
        for ((_, path), target) in staged.iter().zip(&targets) {
            let valid = tokio::select! {_ = cancel.cancelled()=>return Err(Error::Cancelled),result=download::verified(path,target)=>result?};
            if !valid {
                return Err(RuleError::invalid("临时文件校验失败，请重试原任务").into());
            }
        }
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        // 安装前完成整份名单的路径检查，交叉复用路径统一先删除再写入
        let game = paths::game(&self.root, &self.config.game_dir)?;
        for operation in &record.plan.operations {
            if let Some(old) = &operation.old_path {
                paths::checked(&game, old)?;
            }
            if let Some(target) = &operation.target {
                paths::checked(&game, &target.path)?;
            }
        }
        self.phase("installing").await?;
        for operation in &record.plan.operations {
            if let Some(old) = &operation.old_path {
                let path = paths::checked(&game, old)?;
                match tokio::fs::remove_file(path).await {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
        }
        for ((_, stage), target) in staged.iter().zip(&targets) {
            let destination = paths::checked(&game, &target.path)?;
            let parent = destination
                .parent()
                .ok_or_else(|| RuleError::invalid("安装路径无效"))?;
            tokio::fs::create_dir_all(parent).await?;
            let temporary = parent.join(format!(".packy-{}.tmp", uuid::Uuid::new_v4()));
            let mut cleanup = TemporaryFile::new(temporary.clone());
            // 同目标目录复制和替换，staging 在另一磁盘卷也不会触发跨卷 rename
            tokio::fs::copy(stage, &temporary).await?;
            tokio::fs::OpenOptions::new()
                .write(true)
                .open(&temporary)
                .await?
                .sync_all()
                .await?;
            paths::checked(&game, &target.path)?;
            tokio::fs::rename(&temporary, &destination).await?;
            cleanup.keep();
        }
        self.phase("saving_state").await?;
        let installed = LocalState {
            installed_release_id: record.plan.target_release_id.clone(),
        };
        storage::write_json(&self.root, ".updater/state.json", &installed).await?;
        *self.installed.lock().await = installed;
        let mut finished = record.clone();
        finished.phase = "completed".into();
        storage::write_json(&self.root, ".updater/task.json", &finished).await?;
        // 清理只触及本任务已知的暂存文件，清理失败不能撤销已成功保存的安装结果
        for (_, path) in &staged {
            if let Err(error) = tokio::fs::remove_file(path).await {
                tracing::warn!(%error, "清理已完成任务的暂存文件失败");
            }
        }
        if let Err(error) = tokio::fs::remove_dir(&stage).await {
            tracing::warn!(%error, "清理已完成任务的暂存目录失败");
        }
        Ok(())
    }
    /// 阶段检查点必须先持久化，再开始不可取消的实际文件修改
    async fn phase(&self, phase: &str) -> Result<()> {
        let record = {
            let mut inner = self.inner.lock().await;
            if phase == "installing" && inner.cancel.is_cancelled() {
                return Err(Error::Cancelled);
            }
            inner.status.phase = phase.into();
            inner.status.error = None;
            if let Some(record) = &mut inner.record {
                record.phase = phase.into();
            }
            let record = inner.record.clone();
            self.publish(&mut inner);
            record
        };
        if let Some(record) = record {
            storage::write_json(&self.root, ".updater/task.json", &record).await?;
        }
        Ok(())
    }
    /// 每个文件的当前尝试计数独立，重试不把已失败尝试重复计入进度
    async fn bytes(&self, index: usize, bytes: i64) {
        let mut inner = self.inner.lock().await;
        if let Some(current) = inner.progress.get_mut(index) {
            *current = bytes;
        }
        inner.status.downloaded_bytes = inner
            .progress
            .iter()
            .fold(0_i64, |sum, bytes| sum.saturating_add(*bytes));
        inner.status.bytes_per_second = (inner.status.downloaded_bytes as f64
            / inner.started.elapsed().as_secs_f64().max(0.001))
            as i64;
        if inner.last_event.elapsed() >= Duration::from_millis(100) {
            self.publish(&mut inner);
        }
    }
    async fn file_finished(&self) {
        let mut inner = self.inner.lock().await;
        inner.status.completed_files += 1;
        if inner.last_event.elapsed() >= Duration::from_millis(100) {
            self.publish(&mut inner);
        }
    }
}
