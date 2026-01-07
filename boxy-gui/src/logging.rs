use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use chrono::{Duration, Local, NaiveDate};
use serde_json::json;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const LOG_DIR_NAME: &str = "Boxy";
const LOG_FILE_PREFIX: &str = "boxy-";
const LOG_FILE_SUFFIX: &str = ".jsonl";
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const RETENTION_DAYS: i64 = 7;
const MAX_LOG_LINES: usize = 2000;

pub fn init_logging() -> Result<(), String> {
  let log_dir = resolve_log_dir()?;
  ensure_dir(&log_dir)?;
  cleanup_old_logs(&log_dir)?;

  let filter = tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
  let fmt_layer = tracing_subscriber::fmt::layer()
    .with_ansi(false)
    .json()
    .with_writer(move || JsonLogWriter::new(log_dir.clone()));

  tracing_subscriber::registry()
    .with(filter)
    .with(fmt_layer)
    .try_init()
    .map_err(|e| format!("初始化日志失败: {}", e))?;

  Ok(())
}

pub fn append_frontend_log(level: &str, message: &str) -> Result<(), String> {
  let log_dir = resolve_log_dir()?;
  ensure_dir(&log_dir)?;
  cleanup_old_logs(&log_dir)?;

  let line = json!({
    "timestamp": Local::now().to_rfc3339(),
    "level": level,
    "target": "frontend",
    "message": message
  })
  .to_string();

  append_line(&log_dir, &line)?;
  Ok(())
}

pub fn read_logs() -> Result<Vec<String>, String> {
  let log_dir = resolve_log_dir()?;
  if !log_dir.exists() {
    return Ok(Vec::new());
  }

  let mut files = collect_log_files(&log_dir)?;
  files.sort_by(|a, b| b.0.cmp(&a.0));

  let mut lines = Vec::new();
  for (_, path) in files {
    let content = fs::read_to_string(&path)
      .map_err(|e| format!("读取日志失败: {}", e))?;
    for line in content.lines() {
      if line.trim().is_empty() {
        continue;
      }
      lines.push(line.to_string());
      if lines.len() >= MAX_LOG_LINES {
        return Ok(lines);
      }
    }
  }

  Ok(lines)
}

pub fn log_path_display() -> Result<String, String> {
  Ok(resolve_log_dir()?.to_string_lossy().to_string())
}

#[derive(Clone)]
struct JsonLogWriter {
  log_dir: PathBuf,
}

impl JsonLogWriter {
  fn new(log_dir: PathBuf) -> Self {
    Self { log_dir }
  }
}

impl Write for JsonLogWriter {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    if buf.is_empty() {
      return Ok(0);
    }
    if let Err(err) = append_line_bytes(&self.log_dir, buf) {
      eprintln!("写入日志失败: {}", err);
    }
    Ok(buf.len())
  }

  fn flush(&mut self) -> std::io::Result<()> {
    Ok(())
  }
}

fn resolve_log_dir() -> Result<PathBuf, String> {
  if let Some(home) = dirs::home_dir() {
    return Ok(home.join("Library").join("Logs").join(LOG_DIR_NAME));
  }
  if let Some(cache) = dirs::cache_dir() {
    return Ok(cache.join(LOG_DIR_NAME).join("Logs"));
  }
  Err("无法解析日志目录".to_string())
}

fn ensure_dir(path: &Path) -> Result<(), String> {
  fs::create_dir_all(path).map_err(|e| format!("创建日志目录失败: {}", e))
}

fn append_line(log_dir: &Path, line: &str) -> Result<(), String> {
  let mut owned = line.to_string();
  owned.push('\n');
  append_line_bytes(log_dir, owned.as_bytes())
}

fn append_line_bytes(log_dir: &Path, bytes: &[u8]) -> Result<(), String> {
  let path = current_log_file(log_dir);
  if let Ok(metadata) = fs::metadata(&path) {
    if metadata.len() >= MAX_LOG_BYTES {
      return Ok(());
    }
  }

  let mut file = OpenOptions::new()
    .create(true)
    .append(true)
    .open(&path)
    .map_err(|e| format!("打开日志文件失败: {}", e))?;
  file
    .write_all(bytes)
    .map_err(|e| format!("写入日志失败: {}", e))?;
  Ok(())
}

fn current_log_file(log_dir: &Path) -> PathBuf {
  let date = Local::now().format("%Y-%m-%d");
  log_dir.join(format!("{}{}{}", LOG_FILE_PREFIX, date, LOG_FILE_SUFFIX))
}

fn collect_log_files(log_dir: &Path) -> Result<Vec<(NaiveDate, PathBuf)>, String> {
  let mut files = Vec::new();
  let entries = fs::read_dir(log_dir).map_err(|e| format!("读取日志目录失败: {}", e))?;
  for entry in entries {
    let entry = entry.map_err(|e| format!("读取日志条目失败: {}", e))?;
    let path = entry.path();
    if let Some((date, _)) = parse_log_file(&path) {
      files.push((date, path));
    }
  }
  Ok(files)
}

fn parse_log_file(path: &Path) -> Option<(NaiveDate, String)> {
  let file_name = path.file_name()?.to_string_lossy();
  if !file_name.starts_with(LOG_FILE_PREFIX) || !file_name.ends_with(LOG_FILE_SUFFIX) {
    return None;
  }
  let date_str = file_name
    .trim_start_matches(LOG_FILE_PREFIX)
    .trim_end_matches(LOG_FILE_SUFFIX);
  let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
  Some((date, file_name.to_string()))
}

fn cleanup_old_logs(log_dir: &Path) -> Result<(), String> {
  if !log_dir.exists() {
    return Ok(());
  }

  let cutoff = Local::now().date_naive() - Duration::days(RETENTION_DAYS);
  for (date, path) in collect_log_files(log_dir)? {
    if date < cutoff {
      let _ = fs::remove_file(path);
    }
  }

  Ok(())
}
