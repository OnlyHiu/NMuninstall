//! Structured error type shared by every Tauri command.
//!
//! The frontend receives `{ code, message, detail }`; `message` is user-facing
//! Chinese text, `detail` is technical and only ends up in the log file.

use serde::ser::{Serialize, SerializeStruct, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("未找到该程序，列表可能已过期，请刷新后重试")]
    NotFound {
        #[allow(dead_code)]
        id: String,
    },
    #[error("该程序未提供卸载命令，无法卸载")]
    NoUninstallString,
    #[error("无法解析卸载命令：{0}")]
    ParseFailed(String),
    #[error("启动卸载程序失败：{0}")]
    SpawnFailed(String),
    #[error("卸载超时（超过 {0} 秒），已终止卸载程序")]
    UninstallTimeout(u64),
    #[error("注册表读取失败：{0}")]
    RegistryReadFailed(String),
    #[error("路径不在允许操作的白名单内：{0}")]
    PathNotAllowed(String),
    #[error("路径不存在：{0}")]
    PathNotFound(String),
    #[error("清理残留失败：{0}")]
    CleanFailed(String),
    #[error("设置读取失败：{0}")]
    SettingsReadFailed(String),
    #[error("设置写入失败：{0}")]
    SettingsWriteFailed(String),
    #[error("当前平台不支持该操作")]
    NotSupported,
    #[error("内部错误：{0}")]
    Internal(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::NotFound { .. } => "NOT_FOUND",
            AppError::NoUninstallString => "NO_UNINSTALL_STRING",
            AppError::ParseFailed(_) => "PARSE_FAILED",
            AppError::SpawnFailed(_) => "SPAWN_FAILED",
            AppError::UninstallTimeout(_) => "UNINSTALL_TIMEOUT",
            AppError::RegistryReadFailed(_) => "REGISTRY_READ_FAILED",
            AppError::PathNotAllowed(_) => "PATH_NOT_ALLOWED",
            AppError::PathNotFound(_) => "PATH_NOT_FOUND",
            AppError::CleanFailed(_) => "CLEAN_FAILED",
            AppError::SettingsReadFailed(_) => "SETTINGS_READ_FAILED",
            AppError::SettingsWriteFailed(_) => "SETTINGS_WRITE_FAILED",
            AppError::NotSupported => "NOT_SUPPORTED",
            AppError::Internal(_) => "INTERNAL",
        }
    }

    /// Technical detail for the log file; never shown to the user.
    pub fn detail(&self) -> Option<String> {
        match self {
            AppError::NotFound { id } => Some(format!("id={id}")),
            AppError::ParseFailed(d)
            | AppError::SpawnFailed(d)
            | AppError::RegistryReadFailed(d)
            | AppError::CleanFailed(d)
            | AppError::SettingsReadFailed(d)
            | AppError::SettingsWriteFailed(d)
            | AppError::Internal(d) => Some(d.clone()),
            AppError::PathNotAllowed(p) | AppError::PathNotFound(p) => Some(p.clone()),
            _ => None,
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let len = if self.detail().is_some() { 3 } else { 2 };
        let mut st = serializer.serialize_struct("AppError", len)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        if let Some(detail) = self.detail() {
            st.serialize_field("detail", &detail)?;
        }
        st.end()
    }
}

// `winreg` 0.55 surfaces `std::io::Error`, so the blanket conversion below
// already covers every registry call.
impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError::Internal(value.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_without_detail_when_absent() {
        let e = AppError::NoUninstallString;
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["code"], "NO_UNINSTALL_STRING");
        assert!(json.get("detail").is_none());
    }

    #[test]
    fn serializes_detail_when_present() {
        let e = AppError::ParseFailed("missing brace".into());
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["code"], "PARSE_FAILED");
        assert_eq!(json["detail"], "missing brace");
    }

    #[test]
    fn message_is_user_facing_chinese() {
        let e = AppError::PathNotAllowed("C:\\Windows".into());
        assert!(e.to_string().contains("白名单"));
    }
}
