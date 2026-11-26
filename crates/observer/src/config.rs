use std::env;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Settings {
    pub app_name: String,
    pub secret_key: String,
    pub debug: bool,
    pub port: u16,
    pub database_path: String,
    pub access_token_expire_minutes: i64,
    pub refresh_token_expire_days: i64,
    pub max_concurrent_checks: usize,
    /// Login attempts allowed per IP per minute. Zero disables the limiter.
    pub login_rate_limit: u32,
    pub scheduler_enabled: bool,
    pub version: String,
}

impl Settings {
    pub fn from_env() -> Self {
        let port = env_parse("PORT", 3000u16);
        let database_path =
            env::var("DATABASE_PATH").unwrap_or_else(|_| "/data/observer.db".to_string());
        Self {
            app_name: "Observer Lite".to_string(),
            secret_key: env::var("SECRET_KEY")
                .unwrap_or_else(|_| "change-me-in-production-use-a-long-random-string".to_string()),
            debug: env_bool("DEBUG", false),
            port,
            database_path,
            access_token_expire_minutes: env_parse("ACCESS_TOKEN_EXPIRE_MINUTES", 15i64),
            refresh_token_expire_days: env_parse("REFRESH_TOKEN_EXPIRE_DAYS", 30i64),
            max_concurrent_checks: env_parse("MAX_CONCURRENT_CHECKS", 32usize).max(1),
            login_rate_limit: env_parse("LOGIN_RATE_LIMIT", 5u32),
            scheduler_enabled: env_bool("SCHEDULER_ENABLED", true),
            version: read_version(),
        }
    }

    pub fn sqlite_url(&self) -> String {
        format!("sqlite://{}?mode=rwc", self.database_path)
    }

    pub fn for_test(database_path: impl Into<String>) -> Self {
        Self {
            app_name: "Observer Lite".into(),
            secret_key: "test-secret-key".into(),
            debug: false,
            port: 0,
            database_path: database_path.into(),
            access_token_expire_minutes: 15,
            refresh_token_expire_days: 30,
            max_concurrent_checks: 4,
            login_rate_limit: 0,
            scheduler_enabled: false,
            version: "0.3.0-test".into(),
        }
    }
}

fn env_bool(name: &str, default: bool) -> bool {
    match env::var(name) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => default,
    }
}

fn env_parse<T: std::str::FromStr>(name: &str, default: T) -> T {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn read_version() -> String {
    let mut starts = Vec::new();
    if let Ok(cwd) = env::current_dir() {
        starts.push(cwd);
    }
    starts.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    for start in starts {
        let mut dir = start;
        for _ in 0..6 {
            let file = dir.join("VERSION");
            if let Ok(text) = std::fs::read_to_string(&file) {
                let version = text.trim();
                if !version.is_empty() {
                    return version.to_string();
                }
            }
            if !dir.pop() {
                break;
            }
        }
    }
    env!("CARGO_PKG_VERSION").to_string()
}

pub fn ensure_parent(path: &str) -> std::io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_parent_and_environment() {
        let settings = Settings::for_test("observer.db");
        assert!(settings.sqlite_url().starts_with("sqlite://"));
        ensure_parent("observer.db").unwrap();
        let dir = std::env::temp_dir().join("observer-lite-parent");
        let _ = std::fs::remove_dir_all(&dir);
        ensure_parent(dir.join("nested").join("db").to_str().unwrap()).unwrap();

        std::env::set_var("PORT", "3999");
        std::env::set_var("DEBUG", "yes");
        std::env::set_var("MAX_CONCURRENT_CHECKS", "0");
        std::env::set_var("SCHEDULER_ENABLED", "off");
        std::env::set_var("LOGIN_RATE_LIMIT", "nope");
        std::env::set_var("SECRET_KEY", "unit-secret");
        let loaded = Settings::from_env();
        assert_eq!(loaded.port, 3999);
        assert!(loaded.debug);
        assert_eq!(loaded.max_concurrent_checks, 1);
        assert!(!loaded.scheduler_enabled);
        assert_eq!(loaded.secret_key, "unit-secret");
        assert!(!loaded.version.is_empty());
        std::env::remove_var("PORT");
        std::env::remove_var("DEBUG");
        std::env::remove_var("MAX_CONCURRENT_CHECKS");
        std::env::remove_var("SCHEDULER_ENABLED");
        std::env::remove_var("LOGIN_RATE_LIMIT");
        std::env::remove_var("SECRET_KEY");
    }
}
