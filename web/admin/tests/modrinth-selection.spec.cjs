const { test } = require('@playwright/test');
const selectionRegression = require('./modrinth-selection.cjs');
test('快速切换项目后导入保持最后选择的项目及文件', async ({
  page,
}, testInfo) => {
  await page.goto('/');
  await selectionRegression(
    page,
    testInfo.outputPath('modrinth-selection.png'),
  );
});
