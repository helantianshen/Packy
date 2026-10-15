use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn command(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_packy-server"));
    command.current_dir(root);
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("PACKY_") || name == "RUST_LOG" {
            command.env_remove(name);
        }
    }
    command
}

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_http(server: &mut Server, address: &str, path: &str) -> std::io::Result<String> {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if let Some(status) = server.0.try_wait()? {
            return Err(std::io::Error::other(format!("服务提前退出：{status}")));
        }
        if let Ok(mut stream) = TcpStream::connect(address) {
            stream.set_read_timeout(Some(Duration::from_secs(2)))?;
            write!(
                stream,
                "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
            )?;
            let mut response = String::new();
            stream.read_to_string(&mut response)?;
            return Ok(response);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        "等待 HTTP 服务超时",
    ))
}

#[test]
fn yaml_only_and_environment_overrides() -> TestResult {
    for source in ["yaml", "overrides", "environment"] {
        let overrides = source != "yaml";
        let root = tempfile::tempdir()?;
        std::fs::create_dir(root.path().join("config"))?;
        let address = TcpListener::bind("127.0.0.1:0")?.local_addr()?.to_string();
        std::fs::create_dir(root.path().join("admin"))?;
        std::fs::write(root.path().join("admin/index.html"), "yaml-admin-marker")?;
        let yaml = if overrides {
            "listen: invalid\npublic_url: invalid\ndatabase: unused.db\nfiles_dir: unused-files\nadmin_dir: unused-admin\njwt_secret: ''\nmodrinth_user_agent: ''\nlog_filter: '[invalid'\n".into()
        } else {
            format!(
                "listen: '{address}'\npublic_url: 'http://{address}'\ndatabase: yaml.db\nfiles_dir: files\nadmin_dir: admin\njwt_secret: '{}'\nmodrinth_user_agent: Packy-Test/1\nlog_filter: warn\n",
                "x".repeat(64)
            )
        };
        if source != "environment" {
            std::fs::write(root.path().join("config/server.yaml"), yaml)?;
        }
        let mut command = command(root.path());
        if overrides {
            command
                .env("PACKY_LISTEN", &address)
                .env("PACKY_PUBLIC_URL", format!("http://{address}"))
                .env("PACKY_DATABASE", "env.db")
                .env("PACKY_FILES_DIR", "files")
                .env("PACKY_ADMIN_DIR", "admin")
                .env("PACKY_JWT_SECRET", "y".repeat(64))
                .env("PACKY_MODRINTH_USER_AGENT", "Packy-Test/2")
                .env("RUST_LOG", "warn");
        }
        let mut server = Server(
            command
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?,
        );
        let health = wait_http(&mut server, &address, "/healthz")?;
        assert!(health.contains("200 OK") && health.contains("\"status\":\"ok\""));
        let admin = wait_http(&mut server, &address, "/admin/index.html")?;
        assert!(admin.contains("yaml-admin-marker"));
        assert!(
            root.path()
                .join(if overrides { "env.db" } else { "yaml.db" })
                .exists()
        );
        assert!(!root.path().join("unused.db").exists());
    }
    Ok(())
}

#[test]
fn init_admin_uses_explicit_yaml_without_jwt() -> TestResult {
    let root = tempfile::tempdir()?;
    std::fs::create_dir(root.path().join("config"))?;
    std::fs::write(root.path().join("custom.yaml"), "database: custom.db\n")?;
    let mut child = command(root.path())
        .env("PACKY_CONFIG", "custom.yaml")
        .args(["init-admin", "testadmin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("缺少标准输入")?
        .write_all(b"TestPassword123!\n")?;
    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(root.path().join("custom.db").exists());
    assert!(!root.path().join("data/packy.db").exists());
    Ok(())
}

#[test]
fn missing_or_invalid_configuration_fails_without_exposing_secret() -> TestResult {
    let root = tempfile::tempdir()?;
    std::fs::create_dir(root.path().join("config"))?;
    let missing = command(root.path()).output()?;
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("jwt_secret"));
    let explicit = command(root.path())
        .env("PACKY_CONFIG", "missing.yaml")
        .output()?;
    assert!(!explicit.status.success());
    assert!(String::from_utf8_lossy(&explicit.stderr).contains("无法读取配置文件"));
    for yaml in [
        "unknown_field: true\n",
        "jwt_secret: [secret-value]\n",
        "jwt_secret: [\n",
    ] {
        std::fs::write(root.path().join("config/server.yaml"), yaml)?;
        let invalid = command(root.path()).output()?;
        assert!(!invalid.status.success());
        let error = String::from_utf8_lossy(&invalid.stderr);
        assert!(error.contains("配置文件"));
        assert!(!error.contains("secret-value"));
    }
    std::fs::write(
        root.path().join("config/server.yaml"),
        format!("jwt_secret: '{}'\n", "z".repeat(64)),
    )?;
    for (name, value, message) in [
        ("PACKY_JWT_SECRET", "", "jwt_secret"),
        ("PACKY_LISTEN", "invalid", "IP:端口"),
        ("PACKY_PUBLIC_URL", "invalid", "HTTP/HTTPS"),
        ("PACKY_DATABASE", "", "路径不能为空"),
        ("RUST_LOG", "[invalid", "log_filter"),
    ] {
        let invalid = command(root.path()).env(name, value).output()?;
        assert!(!invalid.status.success());
        assert!(String::from_utf8_lossy(&invalid.stderr).contains(message));
    }
    Ok(())
}
