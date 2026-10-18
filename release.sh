#!/usr/bin/env bash
set -euo pipefail

# 从脚本位置定位项目，支持带空格的路径
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
command -v node >/dev/null || { echo '请先安装 .node-version 指定的 Node.js' >&2; exit 1; }
command -v npm >/dev/null || { echo '未找到 npm' >&2; exit 1; }
npm run release -- "$@"
