import { imageSize } from 'image-size';
import { constants } from 'node:fs';
import { lstat, open } from 'node:fs/promises';
import path from 'node:path';

export const brandNames = [
  'logo.png',
  'background-light.webp',
  'background-dark.webp',
  'tray.png',
  'tray@2x.png',
  'trayTemplate.png',
  'trayTemplate@2x.png',
] as const;
export type BrandName = (typeof brandNames)[number];

/** 仅读取固定品牌文件，不跟随目录或文件链接，单文件最多 8 MiB */
export async function readBrandFile(root: string, name: string): Promise<Buffer | undefined> {
  if (!brandNames.includes(name as BrandName)) return undefined;
  try {
    let current = root;
    for (const segment of ['.updater', 'branding']) {
      current = path.join(current, segment);
      const stat = await lstat(current);
      if (!stat.isDirectory() || stat.isSymbolicLink()) return undefined;
    }
    const filename = path.join(current, name);
    const before = await lstat(filename);
    if (!before.isFile() || before.isSymbolicLink() || before.size > 8 * 1024 * 1024)
      return undefined;
    const file = await open(filename, constants.O_RDONLY | (constants.O_NOFOLLOW || 0));
    try {
      const stat = await file.stat();
      if (
        !stat.isFile() ||
        stat.size > 8 * 1024 * 1024 ||
        stat.ino !== before.ino ||
        stat.dev !== before.dev
      )
        return undefined;
      // 固定缓冲区限制读取量，文件在读取期间增长也不能扩大内存占用
      const buffer = Buffer.alloc(8 * 1024 * 1024 + 1);
      let length = 0;
      while (length < buffer.length) {
        const result = await file.read(buffer, length, buffer.length - length, null);
        if (!result.bytesRead) break;
        length += result.bytesRead;
      }
      return length <= 8 * 1024 * 1024 ? buffer.subarray(0, length) : undefined;
    } finally {
      await file.close();
    }
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT')
      console.warn('品牌资源读取失败，使用默认素材', name);
    return undefined;
  }
}

/** 只允许 PNG/WebP 栅格，解码前检查尺寸，不解析其他图像格式 */
export function brandDataURL(bytes: Buffer, name: string): string | undefined {
  const png = bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
  const webp =
    bytes.toString('ascii', 0, 4) === 'RIFF' && bytes.toString('ascii', 8, 12) === 'WEBP';
  if (name.endsWith('.png') ? !png : !webp) return undefined;
  try {
    const size = imageSize(bytes);
    if (
      !size.width ||
      !size.height ||
      size.width > 8192 ||
      size.height > 8192 ||
      size.width * size.height > 32000000
    )
      return undefined;
    return `data:image/${png ? 'png' : 'webp'};base64,${bytes.toString('base64')}`;
  } catch {
    return undefined;
  }
}
