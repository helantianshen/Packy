#!/usr/bin/env python3
"""启动隔离的 HTTP 联调环境，数据仅保存在新建的临时目录中"""

import hashlib
import http.server
import io
import json
import os
from pathlib import Path
import shlex
import signal
import socket
import subprocess
import tempfile
import threading
import time
import urllib.request
import zipfile

REPO = Path(__file__).resolve().parents[1]
ROOT = Path(tempfile.mkdtemp(prefix="packy-smoke-"))
GAME = ROOT / "game"
USER = "smokeadmin"
PASSWORD = "Packy-smoke-2026"


def free_port():
    """仅为本机联调选择空闲端口，不修改已运行服务"""
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def jar(content):
    """生成可识别的测试 ZIP/JAR，不作为可运行的 Minecraft Mod"""
    data = io.BytesIO()
    with zipfile.ZipFile(data, "w", compression=zipfile.ZIP_STORED) as archive:
        archive.writestr("META-INF/MANIFEST.MF", "Manifest-Version: 1.0\n")
        archive.writestr("smoke-data.txt", content)
    return data.getvalue()


PAYLOAD = jar(b"Packy integration fixture\n" * 180000)


class Source(http.server.BaseHTTPRequestHandler):
    """受控慢速文件源，便于观察后台更新与托盘行为"""

    def do_GET(self):
        if self.path != "/demo.jar":
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header("Content-Length", str(len(PAYLOAD)))
        self.send_header("Content-Type", "application/java-archive")
        self.end_headers()
        try:
            for index in range(0, len(PAYLOAD), 65536):
                self.wfile.write(PAYLOAD[index:index + 65536])
                self.wfile.flush()
                time.sleep(0.12)
        except (BrokenPipeError, ConnectionResetError):
            pass

    def log_message(self, *_args):
        pass


def main():
    binary = REPO / "target/debug/packy-server"
    if not binary.exists():
        raise SystemExit("请先执行 cargo build -p packy-server")
    port = free_port()
    base = f"http://127.0.0.1:{port}"
    env = os.environ | {
        "PACKY_DATABASE": str(ROOT / "server.db"),
        "PACKY_FILES_DIR": str(ROOT / "files"),
        "PACKY_ADMIN_DIR": str(REPO / "web/admin/dist"),
        "PACKY_JWT_SECRET": os.urandom(32).hex(),
        "PACKY_PUBLIC_URL": base,
        "PACKY_LISTEN": f"127.0.0.1:{port}",
        "RUST_LOG": "warn",
    }
    subprocess.run([str(binary), "init-admin", USER], input=(PASSWORD + "\n").encode(), env=env, check=True, capture_output=True)
    source = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Source)
    threading.Thread(target=source.serve_forever, daemon=True).start()
    with (ROOT / "server.log").open("w") as log:
        server = subprocess.Popen([str(binary)], cwd=REPO, env=env, stdout=log, stderr=log)
        try:
            for _ in range(100):
                try:
                    with urllib.request.urlopen(base + "/healthz", timeout=1):
                        break
                except OSError:
                    if server.poll() is not None:
                        raise RuntimeError(f"服务启动失败，见 {ROOT / 'server.log'}")
                    time.sleep(0.1)
            token = None

            def request(path, body=None, method=None):
                headers = {"Content-Type": "application/json"}
                if token:
                    headers["Authorization"] = "Bearer " + token
                data = json.dumps(body).encode() if body is not None else None
                req = urllib.request.Request(base + "/api/v1" + path, data=data, headers=headers, method=method)
                with urllib.request.urlopen(req, timeout=30) as response:
                    return json.load(response)

            token = request("/auth/login", {"username": USER, "password": PASSWORD})["access_token"]
            pack = request("/admin/packs", {"name": "绿野旅途 · 联调整合包", "minecraft_version": "1.21.1", "loader": "neoforge"})
            mod = request("/admin/mods", {"modid": "packy_demo", "name": "旅途内容示例", "modrinth_project_id": None, "curseforge_project_id": None})
            first = request(f"/admin/packs/{pack['id']}/drafts", {"version": "1.0.0", "changelog": "初始整合包", "base_release_id": None})
            first = request(f"/admin/packs/{pack['id']}/drafts/{first['id']}/publish", {"revision": first["revision"]})
            artifact = request("/admin/artifacts", {"mod_id": mod["id"], "version": "1.1.0", "filename": "journey-demo.jar", "size": len(PAYLOAD), "sha512": hashlib.sha512(PAYLOAD).hexdigest(), "hosted_file_id": None, "sources": [{"type": "server", "url": f"http://127.0.0.1:{source.server_port}/demo.jar", "priority": 0}]})
            second = request(f"/admin/packs/{pack['id']}/drafts", {"version": "1.1.0", "changelog": "一次轻松的出发。\n\n• 加入旅途内容示例\n• 文件下载后由 SHA-512 校验\n• 自加 Mod 保持不变", "base_release_id": None})
            second = request(f"/admin/packs/{pack['id']}/drafts/{second['id']}", {"revision": second["revision"], "version": "1.1.0", "changelog": second["changelog"], "items": [{"mod_id": mod["id"], "artifact_id": artifact["id"], "target_path": "mods/journey-demo.jar"}]}, "PUT")
            second = request(f"/admin/packs/{pack['id']}/drafts/{second['id']}/publish", {"revision": second["revision"]})
            (GAME / ".updater").mkdir(parents=True)
            (GAME / ".minecraft/mods").mkdir(parents=True)
            (GAME / ".minecraft/mods/player-custom.jar").write_bytes(b"player-owned-untouched")
            (ROOT / "client-build-config.yaml").write_text("".join(f"{key}: {json.dumps(value, ensure_ascii=False)}\n" for key, value in {"pack_name": "蒹葭服务器整合包", "pack_id": pack["id"], "server_url": base, "game_dir": ".minecraft", "launcher_path": "launcher.sh"}.items()))
            (GAME / ".updater/state.json").write_text(json.dumps({"installed_release_id": first["id"]}))
            launcher = GAME / "launcher.sh"
            launcher.write_text("#!/bin/sh\nprintf ok > " + shlex.quote(str(GAME / ".updater/launcher-started")) + "\n")
            launcher.chmod(0o755)
            details = {"root": str(ROOT), "game": str(GAME), "build_config": str(ROOT / "client-build-config.yaml"), "base": base, "admin": base + "/admin/", "username": USER, "password": PASSWORD, "pack_id": pack["id"], "installed_release_id": first["id"], "target_release_id": second["id"]}
            (ROOT / "fixture.json").write_text(json.dumps(details, ensure_ascii=False, indent=2))
            print(json.dumps(details, ensure_ascii=False), flush=True)
            print("客户端联调前将 client-build-config.yaml 的内容填写到项目对应平台的 config/client.<platform>.yaml 并重新构建；不读取整包旁置配置。", flush=True)
            print("仅为隔离测试账号。Ctrl+C 停止服务，临时目录保留供检查。", flush=True)
            while server.poll() is None:
                time.sleep(0.5)
        finally:
            source.shutdown()
            server.terminate()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait()


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, lambda *_args: (_ for _ in ()).throw(KeyboardInterrupt()))
    try:
        main()
    except KeyboardInterrupt:
        pass
