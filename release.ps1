$ErrorActionPreference = 'Stop'

# 保留调用者工作目录，原生命令失败时传递退出码
Push-Location $PSScriptRoot
try {
    if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
        throw '请先安装 .node-version 指定的 Node.js'
    }
    $npm = if ($env:OS -eq 'Windows_NT') { 'npm.cmd' } else { 'npm' }
    if (-not (Get-Command $npm -ErrorAction SilentlyContinue)) {
        throw '未找到 npm'
    }
    & $npm run release -- @args
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}
