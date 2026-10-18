#!/usr/bin/env bash
set +x
set -euo pipefail

usage() {
  cat <<'HELP'
用法：sudo bash deploy.sh
安装至 /opt/packy，systemd 服务名 packy-server
仅用于首次部署，交互输入管理员密码，执行 migrate 和一次 init.sql
已有安装目录或 systemd 服务时拒绝执行，不提供更新功能
自动生成 JWT 密钥并保存；初始化完成后填写公开地址，再手动启动服务
HELP
}
case "${1:-}" in
  --help|-h) usage; exit 0 ;;
  '') ;;
  *) usage >&2; exit 2 ;;
esac
[[ $# -le 1 ]] || { usage >&2; exit 2; }
[[ $(uname -s) == Linux ]] || { echo '仅支持 Linux systemd' >&2; exit 1; }
[[ $EUID -eq 0 ]] || { echo '请使用 sudo bash deploy.sh' >&2; exit 1; }
[[ -d /run/systemd/system ]] || { echo '当前系统未运行 systemd' >&2; exit 1; }
source_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
target=/opt/packy
for command in systemctl install getent useradd groupadd runuser sqlite3 openssl; do
  command -v "$command" >/dev/null || { echo "缺少命令：$command" >&2; exit 1; }
done
for file in packy-server packy-server.service config/server.yaml.example sql/init.sql; do
  [[ -f "$source_dir/$file" ]] || { echo "服务端包缺少：$file" >&2; exit 1; }
done
# 安装目录存在时一律拒绝，失败留下的目录也需维护者检查后处理
if [[ -e "$target" || -L "$target" ]] ||
   [[ $(systemctl show -p LoadState --value packy-server.service) != not-found ]]; then
  echo '已有部署目录或服务，脚本仅供首次部署，不会更新或再次初始化' >&2
  exit 1
fi
"$source_dir/packy-server" --help >/dev/null
# 首次部署使用随包样例的固定数据库路径，避免迁移与 SQL 指向不同数据库
grep -Fxq 'database: data/packy-go.db' "$source_dir/config/server.yaml.example" || {
  echo '部署样例的 database 必须为 data/packy-go.db' >&2; exit 1;
}
# 随机密钥只写入本机配置，不作为命令参数或输出内容
config_template=$(<"$source_dir/config/server.yaml.example")
jwt_field="jwt_secret: ''"
[[ $(grep -c '^jwt_secret:' "$source_dir/config/server.yaml.example") -eq 1 ]] &&
  grep -Fxq "$jwt_field" "$source_dir/config/server.yaml.example" || {
    echo '配置样例必须包含唯一的空 jwt_secret 字段' >&2; exit 1;
  }
jwt_secret=$(openssl rand -hex 32)
[[ "$jwt_secret" =~ ^[0-9a-f]{64}$ ]] || { echo 'JWT 密钥生成失败' >&2; exit 1; }
config_template=${config_template/"$jwt_field"/"jwt_secret: '$jwt_secret'"}
printf '请设置管理员 admin 的密码（8–72 UTF-8 字节）\n' >&2
hash=$("$source_dir/packy-server" password-hash)
[[ "$hash" =~ ^\$2a\$[0-9]{2}\$[./A-Za-z0-9]{53}$ ]] || { echo '密码哈希格式错误' >&2; exit 1; }
if ! getent group packy >/dev/null; then groupadd --system packy; fi
if ! id packy >/dev/null 2>&1; then
  useradd --system --gid packy --home-dir "$target" --no-create-home --shell /usr/sbin/nologin packy
fi
[[ $(id -u packy) -ne 0 ]] || { echo 'packy 用户不能是 root' >&2; exit 1; }
install -d -m 0755 -o root -g root "$target"
install -d -m 0750 -o root -g packy "$target/config" "$target/sql"
install -d -m 0750 -o packy -g packy "$target/data"
install -m 0640 -o root -g packy "$source_dir/config/server.yaml.example" "$target/config/server.yaml.example"
install -m 0640 -o root -g packy "$source_dir/config/server.yaml.example" "$target/config/server.yaml"
printf '%s\n' "$config_template" > "$target/config/server.yaml"
unset jwt_secret config_template
install -m 0640 -o root -g packy "$source_dir/sql/init.sql" "$target/sql/init.sql"
install -m 0755 -o root -g root "$source_dir/packy-server" "$target/packy-server"
cd -- "$target"
# 清除调用者的 PACKY 环境覆盖，迁移和初始化均由服务用户执行
runuser -u packy -- env -i PATH="$PATH" PACKY_CONFIG="$target/config/server.yaml" \
  PACKY_DATABASE="$target/data/packy-go.db" "$target/packy-server" migrate
# 哈希通过标准输入传给 SQLite，不写入参数或额外的 SQL 文件
sql=$(<"$target/sql/init.sql")
[[ "$sql" == *REPLACE_WITH_BCRYPT_HASH* ]] || { echo 'init.sql 缺少密码占位符' >&2; exit 1; }
printf '%s\n' "${sql//REPLACE_WITH_BCRYPT_HASH/$hash}" | \
  runuser -u packy -- sqlite3 -bail "$target/data/packy-go.db"
unset hash sql
install -m 0644 -o root -g root "$source_dir/packy-server.service" /etc/systemd/system/packy-server.service
systemctl daemon-reload
systemctl enable packy-server.service
cat <<'NEXT'
首次部署与数据库初始化完成，管理员用户名为 admin，尚未启动服务
JWT 密钥已自动随机生成并保存至 /opt/packy/config/server.yaml
请编辑该配置中的 public_url，填写客户端可访问的公开地址
数据库已初始化在 /opt/packy/data/packy-go.db，不要再次执行 init.sql
配置完成后启动并检查：
  sudo systemctl start packy-server
  sudo journalctl -u packy-server -n 80 --no-pager
NEXT
