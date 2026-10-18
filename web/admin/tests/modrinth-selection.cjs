module.exports = async (page, screenshotPath) => {
  const writes = [];
  let releaseFirst;
  const firstResponse = new Promise((resolve) => {
    releaseFirst = resolve;
  });
  const origin = new URL(page.url()).origin;
  await page.unroute('**/api/**');
  await page.route(origin + '/api/**', async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    let data = {};
    if (url.pathname.endsWith('/auth/me')) data = { id: 1, username: 'review' };
    else if (url.pathname.endsWith('/modrinth/search'))
      data = {
        hits: ['A', 'B'].map((id) => ({
          project_id: id,
          slug: 'project_' + id.toLowerCase(),
          title: 'Project ' + id,
          description: 'isolated review fixture',
        })),
      };
    else if (url.pathname.endsWith('/versions')) {
      const id = url.pathname.split('/').at(-2);
      if (id === 'A') await firstResponse;
      data = [
        {
          id: 'version' + id,
          version_number: id + '-1',
          game_versions: ['1.21.1'],
          loaders: ['fabric'],
          files: [
            {
              filename: id + '.jar',
              size: 1,
              hashes: { sha512: 'a'.repeat(128) },
              url: 'https://example.invalid/' + id + '.jar',
            },
          ],
        },
      ];
    } else if (request.method() === 'POST') {
      const body = request.postDataJSON();
      writes.push({ path: url.pathname, body });
      data = { id: '33333333-3333-4333-8333-333333333333', ...body };
    } else data = { items: [], total: 0, page: 1, page_size: 100 };
    await route.fulfill({ json: data });
  });
  await page.evaluate(() =>
    sessionStorage.setItem('token', 'isolated-review-token'),
  );
  await page.goto(origin + '/packy/modrinth');
  await page.getByRole('button', { name: '搜索', exact: true }).click();
  const buttons = page.getByRole('button', { name: '选择版本', exact: true });
  await buttons.nth(0).click();
  await buttons.nth(1).click();
  await page.getByText('B.jar', { exact: true }).waitFor();
  releaseFirst();
  await page.waitForTimeout(800);
  if (await page.getByText('A.jar', { exact: true }).count())
    throw Error('stale response replaced project B');
  if (screenshotPath) await page.screenshot({ path: screenshotPath });
  await page.getByRole('button', { name: '导入', exact: true }).click();
  await page.getByText('导入成功', { exact: true }).waitFor();
  if (
    writes[0]?.body.modrinth_project_id !== 'B' ||
    writes[1]?.body.filename !== 'B.jar' ||
    writes[1]?.body.version !== 'B-1'
  )
    throw Error(JSON.stringify(writes));
  return writes;
};
