use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BoxyError {
    #[error("包管理器未找到: {name}")]
    ManagerNotFound { name: String },

    #[error("包管理器不可用: {name}, 原因: {reason}")]
    ManagerUnavailable { name: String, reason: String },

    #[error("包未找到: {manager}/{package}")]
    PackageNotFound { manager: String, package: String },

    #[error("命令执行失败: {manager} '{command}' (退出码: {exit_code})")]
    CommandFailed {
        manager: String,
        command: String,
        exit_code: i32,
    },

    #[error("命令被中断")]
    CommandInterrupted,

    #[error("命令超时")]
    CommandTimeout,

    #[error("解析失败: {input}")]
    ParseError { input: String },

    #[error("JSON解析失败: {message}")]
    JsonError { message: String },

    #[error("缓存错误: {message}")]
    CacheError { message: String },

    #[error("IO错误")]
    Io(#[from] io::Error),

    #[error("网络错误: {message}")]
    NetworkError { message: String },

    #[error("依赖冲突: {message}")]
    DependencyConflict { message: String },

    #[error("不支持的操作: {manager} {operation}")]
    UnsupportedOperation { manager: String, operation: String },
}

pub type Result<T> = std::result::Result<T, BoxyError>;
