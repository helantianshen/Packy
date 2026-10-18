async (page) => {
  const folder = '.impeccable/review/client-b/';
  await page.setViewportSize({ width: 720, height: 560 });
  await page.evaluate(() => {
    window.__state.config.pack_name =
      '蒹葭服务器整合包 · 这是用于检查中文长标题换行与缩放的演示名称';
    window.__state.plan.operations[0].target.path = 'mods/' + 'long-file-name-'.repeat(15) + '.jar';
    window.__state.plan.changelog = '较长的更新说明，用于检查内容换行。'.repeat(12);
    window.__emit('available', { task_id: '', error: undefined });
  });
  await page.getByRole('button', { name: '查看并安装', exact: true }).click();
  await page.waitForTimeout(200);
  const box = await page.locator('dialog').boundingBox();
  if (box.x < 0 || box.y < 0 || box.x + box.width > 720 || box.y + box.height > 560)
    throw Error('确认框超出窗口');
  await page.screenshot({ path: folder + 'long-confirm.png', fullPage: true });
  await page.keyboard.press('Escape');
  if (await page.locator('dialog').isVisible()) throw Error('Escape未关闭确认框');
  await page.getByRole('button', { name: '查看并安装', exact: true }).focus();
  await page.keyboard.press('Enter');
  await page.getByRole('button', { name: '取消', exact: true }).click();
  const action = await page.getByRole('button', { name: '查看并安装', exact: true }).boundingBox();
  if (action.y + action.height > 560) throw Error('主操作不在视口');
  await page.screenshot({ path: folder + 'long-small.png', fullPage: true });
  await page.evaluate(() => window.__emit('downloading', { task_id: 'task-1', total_bytes: null }));
  const progress = page.getByRole('progressbar');
  if (await progress.getAttribute('aria-valuenow')) throw Error('未知大小显示了伪造百分比');
  await page.evaluate(() => {
    window.__state.plan = null;
    window.__emit('checking', { task_id: '', error: undefined });
  });
  await page.getByRole('button', { name: '取消检查', exact: true }).click();
  await page.evaluate(() =>
    window.__emit('failed', { task_id: '', error: { code: 'NETWORK', message: '无法连接服务器' } }),
  );
  if (await page.getByRole('button', { name: '重试原任务', exact: true }).count())
    throw Error('无任务错误提供原任务重试');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.evaluate(() => window.__emit('checking', { error: undefined }));
  const animation = await page
    .locator('.spinning')
    .first()
    .evaluate((el) => getComputedStyle(el).animationName);
  if (animation !== 'none') throw Error('未尊重减少动态效果');
  return {
    passed: true,
    unknownSize: 'indeterminate',
    dialogKeyboard: true,
    primaryVisible: true,
    reducedMotion: animation,
  };
};
