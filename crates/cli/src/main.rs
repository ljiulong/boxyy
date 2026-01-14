use anyhow::{Context, Result};
use boxy_cache::Cache;
use boxy_core::ManagerExecutor;
use boxy_error::BoxyError;
use clap::{Parser, Subcommand};
use colored::*;
#[cfg(target_os = "macos")]
use std::collections::HashSet;
#[cfg(target_os = "macos")]
use std::env;
#[cfg(target_os = "macos")]
use std::process::Command;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};

mod managers;

use managers::{create_manager, supports_global, MANAGER_NAMES};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(300);
const READ_COMMAND_TIMEOUT: Duration = Duration::from_secs(60);
const SCAN_CONCURRENCY: usize = 5;
const EXIT_ERROR: i32 = 1;
const EXIT_USAGE: i32 = 2;

#[derive(Parser, Debug)]
#[command(name = "boxy", version = env!("CARGO_PKG_VERSION"), about = "macOS 统一包管理器")]
struct Cli {
    #[arg(short, long, global = true)]
    json: bool,

    #[arg(short, long, global = true)]
    verbose: bool,

    #[arg(long, global = true)]
    no_cache: bool,

    /// 使用全局包范围（针对 npm、pnpm、yarn、bun）
    #[arg(long, global = true)]
    global: bool,

    /// 包范围（global 或 local）
    #[arg(long, global = true)]
    scope: Option<String>,

    /// 本地范围目录（配合 --scope=local）
    #[arg(long, global = true)]
    dir: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 扫描所有包管理器并列出已安装的包
    Scan {
        /// 只显示可用的包管理器
        #[arg(long)]
        available_only: bool,
    },
    /// 列出已安装的包
    List {
        /// 指定包管理器
        #[arg(short, long)]
        manager: Option<String>,
    },
    /// 查看包详情
    Info {
        /// 包名
        package: String,
        /// 指定包管理器
        #[arg(short, long)]
        manager: Option<String>,
    },
    /// 搜索包
    Search {
        /// 搜索关键词
        query: String,
        /// 指定包管理器
        #[arg(short, long)]
        manager: Option<String>,
    },
    /// 安装包
    Install {
        /// 包名
        package: String,
        /// 版本（可选）
        #[arg(long)]
        version: Option<String>,
        /// 指定包管理器
        #[arg(short, long)]
        manager: Option<String>,
        /// 强制安装
        #[arg(short, long)]
        force: bool,
    },
    /// 更新包
    Update {
        /// 包名（可选，不指定则更新所有）
        package: Option<String>,
        /// 指定包管理器
        #[arg(short, long)]
        manager: Option<String>,
    },
    /// 卸载包
    Uninstall {
        /// 包名
        package: String,
        /// 指定包管理器
        #[arg(short, long)]
        manager: Option<String>,
        /// 强制卸载
        #[arg(short, long)]
        force: bool,
        /// 跳过清理包管理器缓存（默认会清理）
        #[arg(long)]
        keep_cache: bool,
    },
    /// 列出可更新的包
    Outdated {
        /// 指定包管理器
        #[arg(short, long)]
        manager: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    ensure_macos_path();
    let cli = Cli::parse();

    // 初始化 tracing
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    // 创建缓存
    let cache = Arc::new(Cache::new().context("创建缓存失败")?);
    let executor = Arc::new(ManagerExecutor::default());

    // 执行命令
    match cli.command {
        Commands::Scan { available_only } => {
            cmd_scan(
                cache,
                cli.global,
                cli.scope.as_deref(),
                cli.dir.as_deref(),
                available_only,
                cli.json,
                cli.no_cache,
            )
            .await
        }
        Commands::List { manager } => {
            cmd_list(
                cache,
                cli.global,
                cli.scope.as_deref(),
                cli.dir.as_deref(),
                manager.as_deref(),
                cli.json,
                cli.no_cache,
            )
            .await
        }
        Commands::Info { package, manager } => {
            cmd_info(
                cache,
                cli.global,
                cli.scope.as_deref(),
                cli.dir.as_deref(),
                &package,
                manager.as_deref(),
                cli.json,
            )
            .await
        }
        Commands::Search { query, manager } => {
            cmd_search(
                cache,
                cli.global,
                cli.scope.as_deref(),
                cli.dir.as_deref(),
                &query,
                manager.as_deref(),
                cli.json,
            )
            .await
        }
        Commands::Install {
            package,
            version,
            manager,
            force,
        } => {
            cmd_install(
                cache,
                executor.clone(),
                cli.global,
                cli.scope.as_deref(),
                cli.dir.as_deref(),
                &package,
                version.as_deref(),
                manager.as_deref(),
                force,
                cli.json,
            )
            .await
        }
        Commands::Update { package, manager } => {
            cmd_update(
                cache,
                executor.clone(),
                cli.global,
                cli.scope.as_deref(),
                cli.dir.as_deref(),
                package.as_deref(),
                manager.as_deref(),
                cli.json,
            )
            .await
        }
        Commands::Uninstall {
            package,
            manager,
            force,
            keep_cache,
        } => {
            cmd_uninstall(
                cache,
                executor.clone(),
                cli.global,
                cli.scope.as_deref(),
                cli.dir.as_deref(),
                &package,
                manager.as_deref(),
                force,
                !keep_cache,  // 反转逻辑：默认清理，--keep-cache 跳过
                cli.json,
            )
            .await
        }
        Commands::Outdated { manager } => {
            cmd_outdated(
                cache,
                cli.global,
                cli.scope.as_deref(),
                cli.dir.as_deref(),
                manager.as_deref(),
                cli.json,
                cli.no_cache,
            )
            .await
        }
    }
}

fn ensure_macos_path() {
    #[cfg(target_os = "macos")]
    {
        let current = env::var("PATH").unwrap_or_default();
        let mut seen: HashSet<String> = HashSet::new();
        let mut parts: Vec<String> = Vec::new();

        let mut add_path = |value: &str| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return;
            }
            if seen.insert(trimmed.to_string()) {
                parts.push(trimmed.to_string());
            }
        };

        for entry in current.split(':') {
            add_path(entry);
        }

        if let Ok(shell_path) = resolve_login_shell_path() {
            for entry in shell_path.split(':') {
                add_path(entry);
            }
        }

        if let Some(home) = dirs::home_dir() {
            let candidates = [
                home.join(".cargo/bin"),
                home.join(".local/bin"),
                home.join(".pyenv/shims"),
                home.join(".pyenv/bin"),
                home.join(".volta/bin"),
                home.join(".npm-global/bin"),
                home.join("Library/pnpm"),
            ];
            for candidate in candidates {
                if candidate.is_dir() {
                    add_path(&candidate.to_string_lossy());
                }
            }
        }

        for candidate in [
            "/opt/homebrew/bin",
            "/opt/homebrew/sbin",
            "/usr/local/bin",
            "/usr/local/sbin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin",
        ] {
            add_path(candidate);
        }

        if !parts.is_empty() {
            env::set_var("PATH", parts.join(":"));
        }
    }
}

#[cfg(target_os = "macos")]
fn resolve_login_shell_path() -> Result<String, String> {
    let output = Command::new("/bin/zsh")
        .arg("-lc")
        .arg("printf %s \"$PATH\"")
        .output()
        .map_err(|e| format!("无法读取 shell PATH: {}", e))?;
    if !output.status.success() {
        return Err("读取 shell PATH 失败".to_string());
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        return Err("shell PATH 为空".to_string());
    }
    Ok(value)
}

async fn cmd_scan(
    cache: Arc<Cache>,
    global: bool,
    scope: Option<&str>,
    directory: Option<&str>,
    available_only: bool,
    json: bool,
    _no_cache: bool,
) -> Result<()> {
    run_with_timeout("扫描操作超时", async {
    let scope_config = resolve_scope(None, global, scope, directory)?;
    let global = scope_config.global;
    let workdir = scope_config.workdir.clone();
    if json {
        let mut results = Vec::new();
        for name in MANAGER_NAMES.iter() {
            let cache_clone = cache.clone();
            let manager_name = name.to_string();
            let manager = create_manager(&manager_name, cache_clone.clone(), global, workdir.as_ref());
            if let Some(m) = manager {
                let available = m.check_available().await.unwrap_or(false);
                if available_only && !available {
                    continue;
                }

                let packages = if available {
                    if let Err(err) = cache_clone.invalidate(m.cache_key()).await {
                        eprintln!(
                            "{}",
                            format!("错误: 清除 {} 缓存失败: {}", manager_name, err)
                                .bright_red()
                        );
                    }
                    match m.list_installed().await {
                        Ok(list) => list,
                        Err(err) => {
                            eprintln!(
                                "{}",
                                format!("错误: 获取 {} 包列表失败: {}", manager_name, err)
                                    .bright_red()
                            );
                            Vec::new()
                        }
                    }
                } else {
                    Vec::new()
                };

                results.push(serde_json::json!({
                  "manager": manager_name,
                  "available": available,
                  "package_count": packages.len(),
                }));
            }
        }
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        println!("{}", "扫描所有包管理器...".bright_cyan());
        println!();

        // 并行检查所有管理器
        let semaphore = Arc::new(Semaphore::new(SCAN_CONCURRENCY));
        let tasks: Vec<_> = MANAGER_NAMES
            .iter()
            .map(|name| {
                let cache_clone = cache.clone();
                let manager_name = name.to_string();
                let semaphore = semaphore.clone();
                let workdir = workdir.clone();
                tokio::spawn(async move {
                    let _permit = match semaphore.acquire().await {
                        Ok(permit) => permit,
                        Err(_) => return (manager_name, false),
                    };
                    let manager =
                        create_manager(&manager_name, cache_clone, global, workdir.as_ref());
                    if let Some(m) = manager {
                        let available = m.check_available().await.unwrap_or(false);
                        (manager_name, available)
                    } else {
                        (manager_name, false)
                    }
                })
            })
            .collect();

        let mut statuses = Vec::new();
        for task in tasks {
            if let Ok((name, available)) = task.await {
                statuses.push((name, available));
            }
        }

        for (name, available) in statuses {
            if available_only && !available {
                continue;
            }

            let status = if available {
                "✓".bright_green()
            } else {
                "✗".bright_red()
            };
            println!("  {} {}", status, name.bright_white());

            if available {
                let cache_clone = cache.clone();
                let manager = create_manager(&name, cache_clone.clone(), global, workdir.as_ref());
                if let Some(m) = manager {
                    if let Err(err) = cache_clone.invalidate(m.cache_key()).await {
                        eprintln!(
                            "{}",
                            format!("错误: 清除 {} 缓存失败: {}", name, err).bright_red()
                        );
                    }
                    if let Ok(Ok(packages)) =
                        timeout(Duration::from_secs(5), m.list_installed()).await
                    {
                        println!("    {} 已安装包: {}", "→".bright_blue(), packages.len());
                    }
                }
            }
        }
    }

    Ok(())
    })
    .await
}

async fn cmd_list(
    cache: Arc<Cache>,
    global: bool,
    scope: Option<&str>,
    directory: Option<&str>,
    manager_name: Option<&str>,
    json: bool,
    _no_cache: bool,
) -> Result<()> {
    run_with_timeout("列表操作超时", async {
    let scope_config = resolve_scope(manager_name, global, scope, directory)?;
    let global = scope_config.global;
    let workdir = scope_config.workdir.clone();
    let manager_names = resolve_manager_names(manager_name);

    // 并行扫描所有管理器
    let semaphore = Arc::new(Semaphore::new(SCAN_CONCURRENCY));
    let tasks: Vec<_> = manager_names
        .iter()
        .map(|name| {
            let manager_name = name.clone();
            let cache_clone = cache.clone();
            let semaphore = semaphore.clone();
            let workdir = workdir.clone();
            tokio::spawn(async move {
                let _permit = match semaphore.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        return Ok((manager_name, Vec::new(), false));
                    }
                };
                let result: Result<(String, Vec<boxy_core::Package>, bool)> = {
                    let manager =
                        create_manager(&manager_name, cache_clone.clone(), global, workdir.as_ref());
                    if let Some(m) = manager {
                        let available = m.check_available().await.unwrap_or(false);
                        if !available {
                            Ok((manager_name, Vec::new(), false))
                        } else {
                            cache_clone
                                .invalidate(m.cache_key())
                                .await
                                .with_context(|| format!("清除 {} 缓存失败", manager_name))?;
                            let packages = m
                                .list_installed()
                                .await
                                .with_context(|| format!("获取 {} 包列表失败", manager_name))?;

                            Ok((manager_name, packages, true))
                        }
                    } else {
                        Ok((manager_name, Vec::new(), false))
                    }
                };
                result
            })
        })
        .collect();

    let mut all_packages = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok((name, packages, available))) => all_packages.push((name, packages, available)),
            Ok(Err(err)) => return Err(err),
            Err(err) => return Err(anyhow::anyhow!("任务执行失败: {}", err)),
        }
    }

    if json {
        let output: Vec<serde_json::Value> = all_packages
            .into_iter()
            .map(|(manager, packages, available)| {
                serde_json::json!({
                  "manager": manager,
                  "available": available,
                  "packages": packages,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let mut has_output = false;
        for (manager, packages, available) in all_packages {
            if !available {
                println!(
                    "{}",
                    format!("{}: 不可用", manager.bright_cyan()).dimmed()
                );
                has_output = true;
            } else if packages.is_empty() {
                println!(
                    "{}",
                    format!("{}: 没有安装任何包", manager.bright_cyan()).dimmed()
                );
                has_output = true;
            } else {
                println!(
                    "{}",
                    format!("{} ({})", manager.bright_cyan(), packages.len()).bold()
                );
                for pkg in packages {
                    println!("  {} {}", "•".bright_green(), pkg.name.bright_white());
                    if !pkg.version.is_empty() {
                        println!("    {}", format!("版本: {}", pkg.version).dimmed());
                    }
                }
                println!();
                has_output = true;
            }
        }
        if !has_output {
            eprintln!("{}", "错误: 没有找到任何包管理器".bright_red());
            std::process::exit(EXIT_ERROR);
        }
    }

    Ok(())
    })
    .await
}

async fn cmd_info(
    cache: Arc<Cache>,
    global: bool,
    scope: Option<&str>,
    directory: Option<&str>,
    package: &str,
    manager_name: Option<&str>,
    json: bool,
) -> Result<()> {
    run_with_timeout("查询信息超时", async {
    let scope_config = resolve_scope(manager_name, global, scope, directory)?;
    let global = scope_config.global;
    let workdir = scope_config.workdir.clone();
    let manager_names = resolve_manager_names(manager_name);

    // 并行搜索所有管理器
    let semaphore = Arc::new(Semaphore::new(SCAN_CONCURRENCY));
    let tasks: Vec<_> = manager_names
        .iter()
        .map(|name| {
            let manager_name = name.clone();
            let cache_clone = cache.clone();
            let pkg_name = package.to_string();
            let semaphore = semaphore.clone();
            let workdir = workdir.clone();
            tokio::spawn(async move {
                let _permit = match semaphore.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => return Ok(None),
                };
                let manager = create_manager(&manager_name, cache_clone, global, workdir.as_ref());
                if let Some(m) = manager {
                    if !m.check_available().await.unwrap_or(false) {
                        return Ok(None);
                    }

                    let info = m
                        .get_info(&pkg_name)
                        .await
                        .with_context(|| format!("获取 {} 包信息失败", manager_name))?;
                    Ok(Some((manager_name, info)))
                } else {
                    Ok(None)
                }
            })
        })
        .collect();

    let mut results = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok(Some((manager, pkg)))) => results.push((manager, pkg)),
            Ok(Ok(None)) => {}
            Ok(Err(err)) => return Err(err),
            Err(err) => return Err(anyhow::anyhow!("任务执行失败: {}", err)),
        }
    }

    if results.is_empty() {
        eprintln!("{}", format!("错误: 未找到包 '{}'", package).bright_red());
        std::process::exit(EXIT_ERROR);
    }

    if json {
        let output: Vec<serde_json::Value> = results
            .into_iter()
            .map(|(manager, pkg)| {
                serde_json::json!({
                  "manager": manager,
                  "package": pkg,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        for (manager, pkg) in results {
            println!(
                "{}",
                format!("{} ({})", pkg.name.bright_cyan(), manager).bold()
            );
            if !pkg.version.is_empty() {
                println!("  版本: {}", pkg.version.bright_white());
            }
            if let Some(desc) = &pkg.description {
                println!("  描述: {}", desc);
            }
            if let Some(homepage) = &pkg.homepage {
                println!("  主页: {}", homepage.bright_blue());
            }
            if let Some(license) = &pkg.license {
                println!("  许可证: {}", license);
            }
            println!();
        }
    }

    Ok(())
    })
    .await
}

async fn cmd_search(
    cache: Arc<Cache>,
    global: bool,
    scope: Option<&str>,
    directory: Option<&str>,
    query: &str,
    manager_name: Option<&str>,
    json: bool,
) -> Result<()> {
    run_with_timeout("搜索超时", async {
    let scope_config = resolve_scope(manager_name, global, scope, directory)?;
    let global = scope_config.global;
    let workdir = scope_config.workdir.clone();
    let manager_names = resolve_manager_names(manager_name);

    // 并行搜索所有管理器
    let semaphore = Arc::new(Semaphore::new(SCAN_CONCURRENCY));
    let tasks: Vec<_> = manager_names
        .iter()
        .map(|name| {
            let manager_name = name.clone();
            let cache_clone = cache.clone();
            let query_str = query.to_string();
            let semaphore = semaphore.clone();
            let workdir = workdir.clone();
            tokio::spawn(async move {
                let _permit = match semaphore.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => return Ok((manager_name, Vec::new())),
                };
                let manager = create_manager(&manager_name, cache_clone, global, workdir.as_ref());
                if let Some(m) = manager {
                    if !m.check_available().await.unwrap_or(false) {
                        return Ok((manager_name, Vec::new()));
                    }

                    let packages = m
                        .search(&query_str)
                        .await
                        .with_context(|| format!("搜索 {} 失败", manager_name))?;
                    Ok((manager_name, packages))
                } else {
                    Ok((manager_name, Vec::new()))
                }
            })
        })
        .collect();

    let mut all_results = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok((name, packages))) => all_results.push((name, packages)),
            Ok(Err(err)) => return Err(err),
            Err(err) => return Err(anyhow::anyhow!("任务执行失败: {}", err)),
        }
    }

    if json {
        let output: Vec<serde_json::Value> = all_results
            .into_iter()
            .map(|(manager, packages)| {
                serde_json::json!({
                  "manager": manager,
                  "packages": packages,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        for (manager, packages) in all_results {
            if packages.is_empty() {
                continue;
            }

            println!(
                "{}",
                format!("{} ({})", manager.bright_cyan(), packages.len()).bold()
            );
            for pkg in packages.iter().take(10) {
                println!("  {} {}", "•".bright_green(), pkg.name.bright_white());
                if let Some(desc) = &pkg.description {
                    println!("    {}", desc.dimmed());
                }
            }
            if packages.len() > 10 {
                println!("  ... 还有 {} 个结果", packages.len() - 10);
            }
            println!();
        }
    }

    Ok(())
    })
    .await
}

async fn cmd_install(
    cache: Arc<Cache>,
    executor: Arc<ManagerExecutor>,
    global: bool,
    scope: Option<&str>,
    directory: Option<&str>,
    package: &str,
    version: Option<&str>,
    manager_name: Option<&str>,
    force: bool,
    json: bool,
) -> Result<()> {
    let manager_name = match manager_name {
        Some(name) => name,
        None => {
            eprintln!("{}", "错误: 必须指定包管理器".bright_red());
            std::process::exit(EXIT_USAGE);
        }
    };
    let scope_config = resolve_scope(Some(manager_name), global, scope, directory)?;
    let global = scope_config.global;
    let workdir = scope_config.workdir.clone();

    let manager =
        create_manager(manager_name, cache.clone(), global, workdir.as_ref())
            .ok_or_else(|| anyhow::anyhow!("未知的包管理器"))?;

    if !manager.check_available().await.unwrap_or(false) {
        eprintln!(
            "{}",
            format!("错误: 包管理器 '{}' 不可用", manager.name()).bright_red()
        );
        std::process::exit(EXIT_ERROR);
    }

    if !json {
        println!(
            "安装 {} 到 {}...",
            package.bright_white(),
            manager.name().bright_cyan()
        );
    }

    let cache_key = manager.cache_key().to_string();
    let manager_name = manager.name().to_string();
    let workdir = workdir.clone();
    executor
        .execute(&manager_name, || async {
            let manager =
                create_manager(&manager_name, cache.clone(), global, workdir.as_ref())
            .ok_or_else(|| BoxyError::ManagerNotFound {
                name: manager_name.clone(),
            })?;
            timeout(COMMAND_TIMEOUT, manager.install(package, version, force))
                .await
                .map_err(|_| BoxyError::CommandTimeout)?
        })
        .await
        .map_err(|err| anyhow::anyhow!(err))
        .context(format!("安装 {} 失败", package))?;

    cache
        .invalidate(&cache_key)
        .await
        .with_context(|| format!("清除 {} 缓存失败", manager.name()))?;

    if !json {
        println!("{}", "✓ 安装成功".bright_green());
    } else {
        println!(
            "{}",
            serde_json::json!({ "status": "success", "package": package })
        );
    }

    Ok(())
}

async fn cmd_update(
    cache: Arc<Cache>,
    executor: Arc<ManagerExecutor>,
    global: bool,
    scope: Option<&str>,
    directory: Option<&str>,
    package: Option<&str>,
    manager_name: Option<&str>,
    json: bool,
) -> Result<()> {
    if let Some(pkg) = package {
        // 更新单个包
        let manager_name = match manager_name {
            Some(name) => name,
            None => {
                eprintln!("{}", "错误: 必须指定包管理器".bright_red());
                std::process::exit(EXIT_USAGE);
            }
        };
        let scope_config = resolve_scope(Some(manager_name), global, scope, directory)?;
        let global = scope_config.global;
        let workdir = scope_config.workdir.clone();

        let manager =
            create_manager(manager_name, cache.clone(), global, workdir.as_ref())
                .ok_or_else(|| anyhow::anyhow!("未知的包管理器"))?;
        let cache_key = manager.cache_key().to_string();
        let manager_name = manager.name().to_string();
        let workdir = workdir.clone();
        executor
            .execute(&manager_name, || async {
                let manager =
                    create_manager(&manager_name, cache.clone(), global, workdir.as_ref())
                .ok_or_else(|| BoxyError::ManagerNotFound {
                    name: manager_name.clone(),
                })?;
                timeout(COMMAND_TIMEOUT, manager.upgrade(pkg))
                    .await
                    .map_err(|_| BoxyError::CommandTimeout)?
            })
            .await
            .map_err(|err| anyhow::anyhow!(err))
            .context(format!("更新 {} 失败", pkg))?;
        cache
            .invalidate(&cache_key)
            .await
            .with_context(|| format!("清除 {} 缓存失败", manager.name()))?;
        if !json {
            println!("{}", "✓ 更新成功".bright_green());
        } else {
            println!(
                "{}",
                serde_json::json!({ "status": "success", "package": pkg })
            );
        }
    } else {
        // 更新所有可更新的包
        let scope_config = resolve_scope(manager_name, global, scope, directory)?;
        let global = scope_config.global;
        let workdir = scope_config.workdir.clone();
        let manager_names = resolve_manager_names(manager_name);
        let semaphore = Arc::new(Semaphore::new(SCAN_CONCURRENCY));
        let tasks: Vec<_> = manager_names
            .iter()
            .map(|name| {
                let manager_name = name.clone();
                let cache_clone = cache.clone();
                let semaphore = semaphore.clone();
                let workdir = workdir.clone();
                tokio::spawn(async move {
                    let _permit = match semaphore.acquire().await {
                        Ok(permit) => permit,
                        Err(_) => return Ok((manager_name, Vec::new())),
                    };
                    let manager = create_manager(
                        &manager_name,
                        cache_clone.clone(),
                        global,
                        workdir.as_ref(),
                    );
                    if let Some(m) = manager {
                        if !m.check_available().await.unwrap_or(false) {
                            return Ok((manager_name, Vec::new()));
                        }
                        let outdated = m
                            .check_outdated()
                            .await
                            .with_context(|| format!("检查 {} 更新失败", manager_name))?;
                        Ok((manager_name, outdated))
                    } else {
                        Ok((manager_name, Vec::new()))
                    }
                })
            })
            .collect();

        let mut all_outdated = Vec::new();
        for task in tasks {
            match task.await {
                Ok(Ok((name, packages))) => all_outdated.push((name, packages)),
                Ok(Err(err)) => return Err(err),
                Err(err) => return Err(anyhow::anyhow!("任务执行失败: {}", err)),
            }
        }

        let mut updated = Vec::new();
        for (manager_name, packages) in all_outdated {
            if packages.is_empty() {
                continue;
            }
            let manager =
                create_manager(&manager_name, cache.clone(), global, workdir.as_ref())
                    .ok_or_else(|| anyhow::anyhow!("未知的包管理器"))?;
            let cache_key = manager.cache_key().to_string();
            for pkg in packages {
                let workdir = workdir.clone();
                executor
                    .execute(manager.name(), || async {
                        let manager =
                            create_manager(&manager_name, cache.clone(), global, workdir.as_ref())
                        .ok_or_else(|| BoxyError::ManagerNotFound {
                            name: manager_name.clone(),
                        })?;
                        timeout(COMMAND_TIMEOUT, manager.upgrade(&pkg.name))
                            .await
                            .map_err(|_| BoxyError::CommandTimeout)?
                    })
                    .await
                    .map_err(|err| anyhow::anyhow!(err))
                    .context(format!("更新 {} 失败", pkg.name))?;
                updated.push((manager_name.clone(), pkg.name.clone()));
            }
            cache
                .invalidate(&cache_key)
                .await
                .with_context(|| format!("清除 {} 缓存失败", manager.name()))?;
        }

        if updated.is_empty() {
            if !json {
                println!("{}", "✓ 没有可更新的包".bright_green());
            } else {
                println!("{}", serde_json::json!({ "updated": [] }));
            }
        } else if json {
            let output: Vec<serde_json::Value> = updated
                .into_iter()
                .map(|(manager, package)| {
                    serde_json::json!({
                      "manager": manager,
                      "package": package,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("{}", "✓ 批量更新完成".bright_green());
        }
    }

    Ok(())
}

async fn cmd_uninstall(
    cache: Arc<Cache>,
    executor: Arc<ManagerExecutor>,
    global: bool,
    scope: Option<&str>,
    directory: Option<&str>,
    package: &str,
    manager_name: Option<&str>,
    force: bool,
    clean_cache: bool,
    json: bool,
) -> Result<()> {
    let manager_name = match manager_name {
        Some(name) => name,
        None => {
            eprintln!("{}", "错误: 必须指定包管理器".bright_red());
            std::process::exit(EXIT_USAGE);
        }
    };
    let scope_config = resolve_scope(Some(manager_name), global, scope, directory)?;
    let global = scope_config.global;
    let workdir = scope_config.workdir.clone();

    let manager =
        create_manager(manager_name, cache.clone(), global, workdir.as_ref())
            .ok_or_else(|| anyhow::anyhow!("未知的包管理器"))?;

    if !json {
        if force {
            println!(
                "强制卸载 {} 从 {}...",
                package.bright_white(),
                manager.name().bright_cyan()
            );
        } else {
            println!(
                "卸载 {} 从 {}...",
                package.bright_white(),
                manager.name().bright_cyan()
            );
        }
    }

    let cache_key = manager.cache_key().to_string();
    let manager_name = manager.name().to_string();
    let workdir = workdir.clone();
    executor
        .execute(&manager_name, || async {
            let manager =
                create_manager(&manager_name, cache.clone(), global, workdir.as_ref())
            .ok_or_else(|| BoxyError::ManagerNotFound {
                name: manager_name.clone(),
            })?;
            timeout(COMMAND_TIMEOUT, manager.uninstall(package, force))
                .await
                .map_err(|_| BoxyError::CommandTimeout)?
        })
        .await
        .map_err(|err| anyhow::anyhow!(err))
        .context(format!("卸载 {} 失败", package))?;

    cache
        .invalidate(&cache_key)
        .await
        .with_context(|| format!("清除 {} 缓存失败", manager.name()))?;

    // 如果指定了 --clean-cache，清理包管理器缓存
    if clean_cache {
        if !json {
            println!("{}", "正在清理包管理器缓存...".bright_cyan());
        }
        match manager.clean_cache().await {
            Ok(_) => {
                if !json {
                    println!("{}", "✓ 缓存清理成功".bright_green());
                }
            }
            Err(BoxyError::UnsupportedOperation { .. }) => {
                if !json {
                    println!(
                        "{}",
                        format!("⚠ {} 不支持缓存清理", manager_name).bright_yellow()
                    );
                }
            }
            Err(err) => {
                if !json {
                    eprintln!(
                        "{}",
                        format!("⚠ 缓存清理失败: {}", err).bright_yellow()
                    );
                }
            }
        }
    }

    if !json {
        println!("{}", "✓ 卸载成功".bright_green());
    } else {
        println!(
            "{}",
            serde_json::json!({ "status": "success", "package": package })
        );
    }

    Ok(())
}

async fn cmd_outdated(
    cache: Arc<Cache>,
    global: bool,
    scope: Option<&str>,
    directory: Option<&str>,
    manager_name: Option<&str>,
    json: bool,
    no_cache: bool,
) -> Result<()> {
    run_with_timeout("检查更新超时", async {
    let scope_config = resolve_scope(manager_name, global, scope, directory)?;
    let global = scope_config.global;
    let workdir = scope_config.workdir.clone();
    let manager_names = resolve_manager_names(manager_name);

    // 并行检查所有管理器
    let semaphore = Arc::new(Semaphore::new(SCAN_CONCURRENCY));
    let tasks: Vec<_> = manager_names
        .iter()
        .map(|name| {
            let manager_name = name.clone();
            let cache_clone = cache.clone();
            let semaphore = semaphore.clone();
            let workdir = workdir.clone();
            tokio::spawn(async move {
                let _permit = match semaphore.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => return Ok((manager_name, Vec::new())),
                };
                let result: Result<(String, Vec<boxy_core::Package>)> = {
                    let manager =
                        create_manager(&manager_name, cache_clone.clone(), global, workdir.as_ref());
                    if let Some(m) = manager {
                        if !m.check_available().await.unwrap_or(false) {
                            Ok((manager_name, Vec::new()))
                        } else {
                            if no_cache {
                                cache_clone
                                    .invalidate(m.cache_key())
                                    .await
                                    .with_context(|| format!("清除 {} 缓存失败", manager_name))?;
                            }

                            let outdated = m
                                .check_outdated()
                                .await
                                .with_context(|| format!("检查 {} 更新失败", manager_name))?;
                            Ok((manager_name, outdated))
                        }
                    } else {
                        Ok((manager_name, Vec::new()))
                    }
                };
                result
            })
        })
        .collect();

    let mut all_outdated = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok((name, packages))) => all_outdated.push((name, packages)),
            Ok(Err(err)) => return Err(err),
            Err(err) => return Err(anyhow::anyhow!("任务执行失败: {}", err)),
        }
    }

    if json {
        let output: Vec<serde_json::Value> = all_outdated
            .into_iter()
            .map(|(manager, packages)| {
                serde_json::json!({
                  "manager": manager,
                  "packages": packages,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let mut has_outdated = false;
        for (manager, packages) in all_outdated {
            if packages.is_empty() {
                continue;
            }

            has_outdated = true;
            println!(
                "{}",
                format!("{} ({})", manager.bright_cyan(), packages.len()).bold()
            );
            for pkg in packages {
                println!("  {} {}", "•".bright_yellow(), pkg.name.bright_white());
                println!("    当前: {}", pkg.version.dimmed());
                if let Some(latest) = &pkg.latest_version {
                    println!("    最新: {}", latest.bright_green());
                }
            }
            println!();
        }

        if !has_outdated {
            println!("{}", "✓ 所有包都是最新版本".bright_green());
        }
    }

    Ok(())
    })
    .await
}

#[derive(Clone)]
struct ScopeConfig {
    global: bool,
    workdir: Option<PathBuf>,
}

async fn run_with_timeout<F>(message: &'static str, fut: F) -> Result<()>
where
    F: std::future::Future<Output = Result<()>>,
{
    timeout(READ_COMMAND_TIMEOUT, fut)
        .await
        .map_err(|_| anyhow::anyhow!(message))?
}

fn resolve_scope(
    manager_name: Option<&str>,
    global: bool,
    scope: Option<&str>,
    directory: Option<&str>,
) -> Result<ScopeConfig> {
    let scope_value = scope.map(|value| value.to_lowercase());
    if directory.is_some() && scope_value.as_deref() != Some("local") {
        return Err(anyhow::anyhow!("--dir 仅能与 --scope=local 一起使用"));
    }

    let (mut global, workdir) = match scope_value.as_deref() {
        None => (global, None),
        Some("global") => (true, None),
        Some("local") => {
            let dir = directory.unwrap_or("").trim();
            if dir.is_empty() {
                return Err(anyhow::anyhow!("缺少本地目录，请使用 --dir 指定"));
            }
            // 支持 ~ 前缀，避免用户手动展开路径
            let path = if let Some(rest) = dir.strip_prefix("~/") {
                let home =
                    std::env::var("HOME").map_err(|_| anyhow::anyhow!("无法解析用户目录"))?;
                PathBuf::from(home).join(rest)
            } else {
                PathBuf::from(dir)
            };
            if !path.is_dir() {
                return Err(anyhow::anyhow!("目录不存在或不可访问"));
            }
            (false, Some(path))
        }
        Some(other) => return Err(anyhow::anyhow!("不支持的 scope: {}", other)),
    };

    if let Some(name) = manager_name {
        if global && !supports_global(name) {
            eprintln!(
                "{}",
                format!("警告: {} 不支持全局范围，忽略 --global 参数", name).bright_yellow()
            );
            global = false;
        }
        if workdir.is_some() && !supports_global(name) {
            return Err(anyhow::anyhow!("{} 不支持本地范围", name));
        }
    }

    Ok(ScopeConfig { global, workdir })
}

fn resolve_manager_names(manager_name: Option<&str>) -> Vec<String> {
    if let Some(name) = manager_name {
        vec![name.to_string()]
    } else {
        MANAGER_NAMES.iter().map(|name| name.to_string()).collect()
    }
}
