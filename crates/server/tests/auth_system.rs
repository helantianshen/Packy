use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use packy_common::models::Credentials;
use packy_server::{
    api,
    application::{AppState, auth},
    infrastructure::config::Config,
};
use serde_json::{Value, json};
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn request(
    app: axum::Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> Result<(StatusCode, Value), Box<dyn std::error::Error>> {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .oneshot(request.body(Body::from(body.to_string()))?)
        .await?;
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024).await?;
    let value = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body)?
    };
    Ok((status, value))
}

#[tokio::test]
async fn auth_rbac_realtime_settings_and_bootstrap() -> TestResult {
    let temp = tempfile::tempdir()?;
    let config = Config {
        admin_dir: temp.path().join("admin"),
        listen: "127.0.0.1:0".parse()?,
        database: temp.path().join("packy.db"),
        files_dir: temp.path().join("files"),
        public_url: "http://127.0.0.1:8080".into(),
        jwt_secret: "test-secret-32-bytes-only-not-for-production".into(),
        modrinth_user_agent: "Packy/integration-test".into(),
    };
    let state = AppState::open(config.clone()).await?;
    let credentials = || Credentials {
        username: "admin".into(),
        password: "test-password".into(),
    };
    assert!(auth::bootstrap(&state.db, credentials()).await?);
    assert!(
        !auth::bootstrap(
            &state.db,
            Credentials {
                username: "another".into(),
                password: "other-password".into()
            }
        )
        .await?
    );
    let app = api::router(state.clone());
    let (status, value) = request(
        app.clone(),
        "POST",
        "/api/v1/auth/login",
        None,
        json!({"username":"ADMIN","password":"test-password"}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let token = value["access_token"].as_str().ok_or("missing token")?;
    let (status, _) = request(
        app.clone(),
        "GET",
        "/api/v1/admin/system/users",
        None,
        Value::Null,
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let registration = json!({"username":"player","password":"player-password"});
    let (status, value) = request(
        app.clone(),
        "POST",
        "/api/v1/auth/register",
        None,
        registration.clone(),
    )
    .await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(value["code"], "REGISTRATION_CLOSED");
    let (status, _) = request(
        app.clone(),
        "PUT",
        "/api/v1/admin/system/settings",
        Some(token),
        json!({"registration_enabled":true}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = request(
        app.clone(),
        "POST",
        "/api/v1/auth/register",
        None,
        json!({"username":"hacker","password":"player-password","roles":["super_admin"]}),
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, player) = request(
        app.clone(),
        "POST",
        "/api/v1/auth/register",
        None,
        registration.clone(),
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(player["roles"], json!(["user"]));
    assert_eq!(player["permissions"], json!([]));
    assert!(player.get("password_hash").is_none());
    let (status, _) = request(
        app.clone(),
        "POST",
        "/api/v1/auth/register",
        None,
        registration.clone(),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    let (_, value) = request(
        app.clone(),
        "GET",
        "/api/v1/auth/me",
        Some(token),
        Value::Null,
    )
    .await?;
    let admin_id = value["id"].as_str().ok_or("missing account")?;
    // 正确身份也必须通过签名、算法、有效期、签发方和受众校验
    let now = chrono::Utc::now().timestamp();
    for case in [
        "signature",
        "algorithm",
        "expired",
        "issuer",
        "audience",
        "future",
    ] {
        let mut claims = json!({
            "sub": admin_id, "iss": "packy", "aud": "packy-admin",
            "iat": now, "exp": now + 1800, "jti": "test", "token_version": 0
        });
        match case {
            "expired" => claims["exp"] = json!(now - 10),
            "issuer" => claims["iss"] = json!("other"),
            "audience" => claims["aud"] = json!("other"),
            "future" => claims["iat"] = json!(now + 300),
            _ => {}
        }
        let algorithm = if case == "algorithm" {
            jsonwebtoken::Algorithm::HS384
        } else {
            jsonwebtoken::Algorithm::HS256
        };
        let secret = if case == "signature" {
            "incorrect-secret"
        } else {
            &config.jwt_secret
        };
        let invalid = jsonwebtoken::encode(
            &jsonwebtoken::Header::new(algorithm),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
        )?;
        let (status, _) = request(
            app.clone(),
            "GET",
            "/api/v1/auth/me",
            Some(&invalid),
            Value::Null,
        )
        .await?;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{case}");
    }

    let (status, _) = request(
        app.clone(),
        "PATCH",
        &format!("/api/v1/admin/system/users/{admin_id}"),
        Some(token),
        json!({"enabled":false,"roles":["user"]}),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = request(
        app.clone(),
        "PUT",
        "/api/v1/admin/system/roles/super_admin/permissions",
        Some(token),
        json!({"permissions":[]}),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = request(
        app.clone(),
        "PUT",
        "/api/v1/admin/system/roles/user/permissions",
        Some(token),
        json!({"permissions":["source:read","user:read"]}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let (_, login) = request(
        app.clone(),
        "POST",
        "/api/v1/auth/login",
        None,
        registration,
    )
    .await?;
    let player_token = login["access_token"]
        .as_str()
        .ok_or("missing player token")?;
    let (status, sources) = request(
        app.clone(),
        "GET",
        "/api/v1/admin/system/download-source-types",
        Some(player_token),
        Value::Null,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sources.as_array().ok_or("missing sources")?.len(), 4);
    let (status, _) = request(
        app.clone(),
        "GET",
        "/api/v1/admin/system/users",
        Some(player_token),
        Value::Null,
    )
    .await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // 权限撤销直接作用于既有 Token，禁用账号同样立即拒绝
    let (status, _) = request(
        app.clone(),
        "PUT",
        "/api/v1/admin/system/roles/user/permissions",
        Some(token),
        json!({"permissions":[]}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = request(
        app.clone(),
        "GET",
        "/api/v1/admin/system/download-source-types",
        Some(player_token),
        Value::Null,
    )
    .await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let player_id = player["id"].as_str().ok_or("missing player")?;
    let (status, _) = request(
        app.clone(),
        "PATCH",
        &format!("/api/v1/admin/system/users/{player_id}"),
        Some(token),
        json!({"enabled":false,"roles":["user"]}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = request(
        app.clone(),
        "GET",
        "/api/v1/auth/me",
        Some(player_token),
        Value::Null,
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = request(
        app.clone(),
        "PUT",
        "/api/v1/admin/system/settings",
        Some(token),
        json!({"registration_enabled":false}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = request(
        app.clone(),
        "POST",
        "/api/v1/auth/register",
        None,
        json!({"username":"late_player","password":"player-password"}),
    )
    .await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = request(
        app.clone(),
        "POST",
        "/api/v1/auth/logout",
        Some(token),
        json!({}),
    )
    .await?;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = request(app, "GET", "/api/v1/auth/me", Some(token), Value::Null).await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // 重开数据库验证种子不覆盖已经修改的设置和权限
    let reopened = AppState::open(config).await?;
    assert!(
        !packy_server::application::system::settings(&reopened.db)
            .await?
            .registration_enabled
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM accounts")
            .fetch_one(&reopened.db.pool)
            .await?,
        2
    );
    Ok(())
}
