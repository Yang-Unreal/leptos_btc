use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// 共享类型 — 前后端均可使用
// ============================================================================

/// 认证相关错误
#[derive(Debug, Clone)]
pub enum AuthError {
    InvalidCredentials,
    UserNotFound,
    HashError(String),
    DuplicateUser(String),
    DatabaseError(String),
}

impl core::fmt::Display for AuthError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AuthError::InvalidCredentials => write!(f, "Invalid username or password"),
            AuthError::UserNotFound => write!(f, "User not found"),
            AuthError::HashError(e) => write!(f, "Password error: {e}"),
            AuthError::DuplicateUser(field) => write!(f, "User with that {field} already exists"),
            AuthError::DatabaseError(e) => write!(f, "Database error: {e}"),
        }
    }
}

impl std::error::Error for AuthError {}

/// 用户模型 — 前后端共享
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    #[cfg_attr(feature = "ssr", sqlx(rename = "password_hash"))]
    #[serde(skip)]
    pub password_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ============================================================================
// SSR-only — 密码哈希 & 认证后端
// ============================================================================

#[cfg(feature = "ssr")]
mod ssr_impl {
    use super::*;
    use argon2::{
        password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
        Argon2,
    };
    use rand_core::OsRng;
    use sqlx::PgPool;

    /// 对明文密码进行 Argon2 哈希
    pub fn hash_password(plaintext: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(plaintext.as_bytes(), &salt)
            .map_err(|e| AuthError::HashError(e.to_string()))?;
        Ok(hash.to_string())
    }

    /// 验证明文密码与哈希是否匹配
    pub fn verify_password(plaintext: &str, hash: &str) -> Result<(), AuthError> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| AuthError::HashError(e.to_string()))?;
        Argon2::default()
            .verify_password(plaintext.as_bytes(), &parsed_hash)
            .map_err(|e| AuthError::HashError(e.to_string()))?;
        Ok(())
    }

    // ---- axum-login trait 实现 ----

    /// 认证会话类型别名
    pub type AuthSession = axum_login::AuthSession<AuthBackend>;

    impl axum_login::AuthUser for User {
        type Id = Uuid;

        fn id(&self) -> Self::Id {
            self.id
        }

        /// session_auth_hash 用于会话校验：
        /// 当密码变更时，哈希值不同，旧 session 自动失效
        fn session_auth_hash(&self) -> &[u8] {
            self.password_hash.as_bytes()
        }
    }

    /// 认证后端 — 持有数据库连接池
    #[derive(Clone, Debug)]
    pub struct AuthBackend {
        pub pool: PgPool,
    }

    impl axum_login::AuthnBackend for AuthBackend {
        type User = User;
        type Credentials = (String, String); // (username_or_email, password)
        type Error = std::convert::Infallible;

        async fn authenticate(
            &self,
            creds: Self::Credentials,
        ) -> Result<Option<Self::User>, Self::Error> {
            let (login, password) = creds;

            // 支持用 username 或 email 登录
            let user = sqlx::query_as::<_, User>(
                "SELECT id, username, email, password_hash, created_at
                 FROM users
                 WHERE username = $1 OR email = $1",
            )
            .bind(&login)
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None);

            match user {
                Some(u) if verify_password(&password, &u.password_hash).is_ok() => Ok(Some(u)),
                _ => Ok(None),
            }
        }

        async fn get_user(
            &self,
            user_id: &axum_login::UserId<Self>,
        ) -> Result<Option<Self::User>, Self::Error> {
            let user = sqlx::query_as::<_, User>(
                "SELECT id, username, email, password_hash, created_at
                 FROM users WHERE id = $1",
            )
            .bind(*user_id)
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None);
            Ok(user)
        }
    }
}

#[cfg(feature = "ssr")]
pub use ssr_impl::*;

// ============================================================================
// Server Functions — 认证 API
// ============================================================================

/// 用户注册
#[server(Register, "/api/auth/register")]
pub async fn register(
    username: String,
    email: String,
    password: String,
) -> Result<(), ServerFnError> {
    use crate::auth::ssr_impl::hash_password;

    let pool = expect_context::<sqlx::PgPool>();
    let username = username.trim().to_lowercase();
    let email = email.trim().to_lowercase();

    // 基础校验
    if username.is_empty() || username.len() > 64 {
        return Err(ServerFnError::new("Username must be 1–64 characters"));
    }
    if email.is_empty() || !email.contains('@') {
        return Err(ServerFnError::new("Invalid email address"));
    }
    if password.len() < 8 {
        return Err(ServerFnError::new("Password must be at least 8 characters"));
    }

    // 检查用户名/邮箱唯一性
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 OR email = $2)",
    )
    .bind(&username)
    .bind(&email)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    if exists {
        return Err(ServerFnError::new("Username or email already taken"));
    }

    let password_hash = hash_password(&password).map_err(|e| ServerFnError::new(e.to_string()))?;

    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::now_v7())
    .bind(&username)
    .bind(&email)
    .bind(&password_hash)
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(())
}

/// 用户登录
#[server(Login, "/api/auth/login")]
pub async fn login(username: String, password: String) -> Result<(), ServerFnError> {
    use crate::auth::ssr_impl::AuthSession;
    use leptos_axum::extract;

    let mut auth: AuthSession = extract().await?;

    let user = auth
        .authenticate((username, password))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    match user {
        Some(user) => {
            auth.login(&user)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            Ok(())
        }
        None => Err(ServerFnError::new("Invalid username or password")),
    }
}

/// 用户登出
#[server(Logout, "/api/auth/logout")]
pub async fn logout() -> Result<(), ServerFnError> {
    use crate::auth::ssr_impl::AuthSession;
    use leptos_axum::extract;

    let mut auth: AuthSession = extract().await?;
    auth.logout()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// 获取当前登录用户
#[server(CurrentUser, "/api/auth/me")]
pub async fn get_current_user() -> Result<Option<User>, ServerFnError> {
    use crate::auth::ssr_impl::AuthSession;
    use leptos_axum::extract;

    let auth: AuthSession = extract().await?;
    Ok(auth.user)
}
