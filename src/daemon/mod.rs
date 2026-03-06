use crate::config::Config;
use crate::agent::{ChairmanAgent, ChairmanConfig};
use crate::instance::{InstanceManager, ConfigManager};
use anyhow::{bail, Result};
use chrono::Utc;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::Duration;

const STATUS_FLUSH_SECONDS: u64 = 5;

pub async fn run(config: Config, host: String, port: u16) -> Result<()> {
    // Pre-flight: check if port is already in use by another multiclaw daemon
    if let Err(_e) = check_port_available(&host, port).await {
        // Port is in use - check if it's our daemon
        if is_multiclaw_daemon_running(&host, port).await {
            tracing::info!("MultiClaw daemon already running on {host}:{port}");
            println!("✓ MultiClaw daemon already running on http://{host}:{port}");
            println!("  Use 'multiclaw restart' to restart, or 'multiclaw status' to check health.");
            return Ok(());
        }
        // Something else is using the port
        bail!(
            "Port {port} is already in use by another process. \
             Run 'lsof -i :{port}' to identify it, or use a different port."
        );
    }

    let initial_backoff = config.reliability.channel_initial_backoff_secs.max(1);
    let max_backoff = config
        .reliability
        .channel_max_backoff_secs
        .max(initial_backoff);

    crate::health::mark_component_ok("daemon");

    if config.heartbeat.enabled {
        let _ =
            crate::heartbeat::engine::HeartbeatEngine::ensure_heartbeat_file(&config.workspace_dir)
                .await;
    }

    let mut handles: Vec<JoinHandle<()>> = vec![spawn_state_writer(config.clone())];

    // 初始化实例管理器和配置管理器
    let instance_manager = Arc::new(InstanceManager::new());
    let config_manager = Arc::new(ConfigManager::new(config.workspace_dir.clone()).await
        .map_err(|e| anyhow::anyhow!("Failed to create ConfigManager: {}", e))?);

    // 尝试加载董事长配置文件，如果不存在则使用默认配置
    let chairman_config_path = config.workspace_dir.join("chairman_config.toml");
    let chairman_config = if chairman_config_path.exists() {
        match ChairmanConfig::from_file(&chairman_config_path).await {
            Ok(cfg) => {
                tracing::info!("Loaded chairman config from {}", chairman_config_path.display());
                cfg
            }
            Err(e) => {
                tracing::warn!("Failed to load chairman config, using defaults: {}", e);
                ChairmanConfig::default()
            }
        }
    } else {
        tracing::info!("No chairman config found, using defaults");
        let default_config = ChairmanConfig::default();
        // 保存默认配置以便用户可以修改
        if let Err(e) = default_config.save_to_file(&chairman_config_path).await {
            tracing::warn!("Failed to save default chairman config: {}", e);
        }
        default_config
    };

    // 确保董事长的核心文件存在（IDENTITY.md, SOUL.md, AGENTS.md 等）
    // 董事长文件应该放在 config_dir（如 ~/.multiclaw），而不是 workspace_dir
    let config_dir = config.config_path.parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| config.workspace_dir.clone());
    if let Err(e) = ensure_chairman_files(&config_dir, &config.workspace_dir).await {
        tracing::warn!("Failed to ensure chairman files: {}", e);
    }

    // 初始化董事长 Agent（用户分身）
    let chairman = Arc::new(ChairmanAgent::initialize_with_config(
        chairman_config,
        format!("user_{}", uuid::Uuid::new_v4()), // TODO: 从配置或用户输入获取实际用户ID
        config.gateway.host.clone(), // 使用 gateway host 作为默认渠道
        instance_manager.clone(),
        config_manager.clone(),
    ).await.map_err(|e| anyhow::anyhow!("Failed to initialize Chairman Agent: {}", e))?);

    tracing::info!(
        "Chairman Agent initialized: name={}, user_id={}",
        chairman.name,
        chairman.user_id
    );

    {
        let gateway_cfg = config.clone();
        let gateway_host = host.clone();
        let chairman_clone = chairman.clone(); // 克隆董事长 Agent 引用
        handles.push(spawn_component_supervisor(
            "gateway",
            initial_backoff,
            max_backoff,
            move || {
                let cfg = gateway_cfg.clone();
                let host = gateway_host.clone();
                let chairman_inner = chairman_clone.clone();
                async move { 
                    // 将董事长 Agent 注入到网关配置中（如果需要）
                    crate::gateway::run_gateway(&host, port, cfg).await 
                }
            },
        ));
    }

    {
        if has_supervised_channels(&config) {
            let channels_cfg = config.clone();
            handles.push(spawn_component_supervisor(
                "channels",
                initial_backoff,
                max_backoff,
                move || {
                    let cfg = channels_cfg.clone();
                    async move { crate::channels::start_channels(cfg).await }
                },
            ));
        } else {
            crate::health::mark_component_ok("channels");
            tracing::info!("No real-time channels configured; channel supervisor disabled");
        }
    }

    if config.heartbeat.enabled {
        let heartbeat_cfg = config.clone();
        handles.push(spawn_component_supervisor(
            "heartbeat",
            initial_backoff,
            max_backoff,
            move || {
                let cfg = heartbeat_cfg.clone();
                async move { Box::pin(run_heartbeat_worker(cfg)).await }
            },
        ));
    }

    if config.cron.enabled {
        let scheduler_cfg = config.clone();
        handles.push(spawn_component_supervisor(
            "scheduler",
            initial_backoff,
            max_backoff,
            move || {
                let cfg = scheduler_cfg.clone();
                async move { crate::cron::scheduler::run(cfg).await }
            },
        ));
    } else {
        crate::health::mark_component_ok("scheduler");
        tracing::info!("Cron disabled; scheduler supervisor not started");
    }

    println!("🧠 MultiClaw daemon started");
    println!("   Gateway:  http://{host}:{port}");
    println!("   Components: gateway, channels, heartbeat, scheduler");
    println!("   Ctrl+C to stop");

    tokio::signal::ctrl_c().await?;
    crate::health::mark_component_error("daemon", "shutdown requested");

    for handle in &handles {
        handle.abort();
    }
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}

pub fn state_file_path(config: &Config) -> PathBuf {
    config
        .config_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join("daemon_state.json")
}

fn spawn_state_writer(config: Config) -> JoinHandle<()> {
    tokio::spawn(async move {
        let path = state_file_path(&config);
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let mut interval = tokio::time::interval(Duration::from_secs(STATUS_FLUSH_SECONDS));
        loop {
            interval.tick().await;
            let mut json = crate::health::snapshot_json();
            if let Some(obj) = json.as_object_mut() {
                obj.insert(
                    "written_at".into(),
                    serde_json::json!(Utc::now().to_rfc3339()),
                );
            }
            let data = serde_json::to_vec_pretty(&json).unwrap_or_else(|_| b"{}".to_vec());
            let _ = tokio::fs::write(&path, data).await;
        }
    })
}

fn spawn_component_supervisor<F, Fut>(
    name: &'static str,
    initial_backoff_secs: u64,
    max_backoff_secs: u64,
    mut run_component: F,
) -> JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    tokio::spawn(async move {
        let mut backoff = initial_backoff_secs.max(1);
        let max_backoff = max_backoff_secs.max(backoff);

        loop {
            crate::health::mark_component_ok(name);
            match run_component().await {
                Ok(()) => {
                    crate::health::mark_component_error(name, "component exited unexpectedly");
                    tracing::warn!("Daemon component '{name}' exited unexpectedly");
                    // Clean exit — reset backoff since the component ran successfully
                    backoff = initial_backoff_secs.max(1);
                }
                Err(e) => {
                    crate::health::mark_component_error(name, e.to_string());
                    tracing::error!("Daemon component '{name}' failed: {e}");
                }
            }

            crate::health::bump_component_restart(name);
            tokio::time::sleep(Duration::from_secs(backoff)).await;
            // Double backoff AFTER sleeping so first error uses initial_backoff
            backoff = backoff.saturating_mul(2).min(max_backoff);
        }
    })
}

async fn run_heartbeat_worker(config: Config) -> Result<()> {
    let observer: std::sync::Arc<dyn crate::observability::Observer> =
        std::sync::Arc::from(crate::observability::create_observer(&config.observability));
    let engine = crate::heartbeat::engine::HeartbeatEngine::new(
        config.heartbeat.clone(),
        config.workspace_dir.clone(),
        observer,
    );
    let delivery = heartbeat_delivery_target(&config)?;

    let interval_mins = config.heartbeat.interval_minutes.max(5);
    let mut interval = tokio::time::interval(Duration::from_secs(u64::from(interval_mins) * 60));

    loop {
        interval.tick().await;

        let file_tasks = engine.collect_tasks().await?;
        let tasks = heartbeat_tasks_for_tick(file_tasks, config.heartbeat.message.as_deref());
        if tasks.is_empty() {
            continue;
        }

        for task in tasks {
            let prompt = format!("[Heartbeat Task] {task}");
            let temp = config.default_temperature;
            match crate::agent::run(
                config.clone(),
                Some(prompt),
                None,
                None,
                temp,
                vec![],
                false,
            )
            .await
            {
                Ok(output) => {
                    crate::health::mark_component_ok("heartbeat");
                    let announcement = if output.trim().is_empty() {
                        "heartbeat task executed".to_string()
                    } else {
                        output
                    };
                    if let Some((channel, target)) = &delivery {
                        if let Err(e) = crate::cron::scheduler::deliver_announcement(
                            &config,
                            channel,
                            target,
                            &announcement,
                        )
                        .await
                        {
                            crate::health::mark_component_error(
                                "heartbeat",
                                format!("delivery failed: {e}"),
                            );
                            tracing::warn!("Heartbeat delivery failed: {e}");
                        }
                    }
                }
                Err(e) => {
                    crate::health::mark_component_error("heartbeat", e.to_string());
                    tracing::warn!("Heartbeat task failed: {e}");
                }
            }
        }
    }
}

fn heartbeat_tasks_for_tick(
    file_tasks: Vec<String>,
    fallback_message: Option<&str>,
) -> Vec<String> {
    if !file_tasks.is_empty() {
        return file_tasks;
    }

    fallback_message
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(|message| vec![message.to_string()])
        .unwrap_or_default()
}

fn heartbeat_delivery_target(config: &Config) -> Result<Option<(String, String)>> {
    let channel = config
        .heartbeat
        .target
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let target = config
        .heartbeat
        .to
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match (channel, target) {
        (None, None) => Ok(None),
        (Some(_), None) => anyhow::bail!("heartbeat.to is required when heartbeat.target is set"),
        (None, Some(_)) => anyhow::bail!("heartbeat.target is required when heartbeat.to is set"),
        (Some(channel), Some(target)) => {
            validate_heartbeat_channel_config(config, channel)?;
            Ok(Some((channel.to_string(), target.to_string())))
        }
    }
}

fn validate_heartbeat_channel_config(config: &Config, channel: &str) -> Result<()> {
    match channel.to_ascii_lowercase().as_str() {
        "telegram" => {
            if config.channels_config.telegram.is_none() {
                anyhow::bail!(
                    "heartbeat.target is set to telegram but channels_config.telegram is not configured"
                );
            }
        }
        "discord" => {
            if config.channels_config.discord.is_none() {
                anyhow::bail!(
                    "heartbeat.target is set to discord but channels_config.discord is not configured"
                );
            }
        }
        "slack" => {
            if config.channels_config.slack.is_none() {
                anyhow::bail!(
                    "heartbeat.target is set to slack but channels_config.slack is not configured"
                );
            }
        }
        "mattermost" => {
            if config.channels_config.mattermost.is_none() {
                anyhow::bail!(
                    "heartbeat.target is set to mattermost but channels_config.mattermost is not configured"
                );
            }
        }
        other => anyhow::bail!("unsupported heartbeat.target channel: {other}"),
    }

    Ok(())
}

fn has_supervised_channels(config: &Config) -> bool {
    config
        .channels_config
        .channels_except_webhook()
        .iter()
        .any(|(_, ok)| *ok)
}

/// Check if a port is available for binding
async fn check_port_available(host: &str, port: u16) -> Result<()> {
    let addr: std::net::SocketAddr = format!("{host}:{port}").parse()?;
    match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            // Successfully bound - close it and return Ok
            drop(listener);
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            bail!("Port {} is already in use", port)
        }
        Err(e) => bail!("Failed to check port {}: {}", port, e),
    }
}

/// Check if a running daemon on this port is our multiclaw daemon
async fn is_multiclaw_daemon_running(host: &str, port: u16) -> bool {
    let url = format!("http://{}:{}/health", host, port);
    match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(client) => match client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    // Check if response looks like our health endpoint
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        // Our health endpoint has "status" and "runtime.components"
                        json.get("status").is_some() && json.get("runtime").is_some()
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Err(_) => false,
        },
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config(tmp: &TempDir) -> Config {
        let config = Config {
            workspace_dir: tmp.path().join("workspace"),
            config_path: tmp.path().join("config.toml"),
            ..Config::default()
        };
        std::fs::create_dir_all(&config.workspace_dir).unwrap();
        config
    }

    #[test]
    fn state_file_path_uses_config_directory() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);

        let path = state_file_path(&config);
        assert_eq!(path, tmp.path().join("daemon_state.json"));
    }

    #[tokio::test]
    async fn supervisor_marks_error_and_restart_on_failure() {
        let handle = spawn_component_supervisor("daemon-test-fail", 1, 1, || async {
            anyhow::bail!("boom")
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();
        let _ = handle.await;

        let snapshot = crate::health::snapshot_json();
        let component = &snapshot["components"]["daemon-test-fail"];
        assert_eq!(component["status"], "error");
        assert!(component["restart_count"].as_u64().unwrap_or(0) >= 1);
        assert!(component["last_error"]
            .as_str()
            .unwrap_or("")
            .contains("boom"));
    }

    #[tokio::test]
    async fn supervisor_marks_unexpected_exit_as_error() {
        let handle = spawn_component_supervisor("daemon-test-exit", 1, 1, || async { Ok(()) });

        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();
        let _ = handle.await;

        let snapshot = crate::health::snapshot_json();
        let component = &snapshot["components"]["daemon-test-exit"];
        assert_eq!(component["status"], "error");
        assert!(component["restart_count"].as_u64().unwrap_or(0) >= 1);
        assert!(component["last_error"]
            .as_str()
            .unwrap_or("")
            .contains("component exited unexpectedly"));
    }

    #[test]
    fn detects_no_supervised_channels() {
        let config = Config::default();
        assert!(!has_supervised_channels(&config));
    }

    #[test]
    fn detects_supervised_channels_present() {
        let mut config = Config::default();
        config.channels_config.telegram = Some(crate::config::TelegramConfig {
            bot_token: "token".into(),
            allowed_users: vec![],
            stream_mode: crate::config::StreamMode::default(),
            draft_update_interval_ms: 1000,
            interrupt_on_new_message: false,
            mention_only: false,
            group_reply: None,
            base_url: None,
        });
        assert!(has_supervised_channels(&config));
    }

    #[test]
    fn detects_dingtalk_as_supervised_channel() {
        let mut config = Config::default();
        config.channels_config.dingtalk = Some(crate::config::schema::DingTalkConfig {
            client_id: "client_id".into(),
            client_secret: "client_secret".into(),
            allowed_users: vec!["*".into()],
        });
        assert!(has_supervised_channels(&config));
    }

    #[test]
    fn detects_mattermost_as_supervised_channel() {
        let mut config = Config::default();
        config.channels_config.mattermost = Some(crate::config::schema::MattermostConfig {
            url: "https://mattermost.example.com".into(),
            bot_token: "token".into(),
            channel_id: Some("channel-id".into()),
            allowed_users: vec!["*".into()],
            thread_replies: Some(true),
            mention_only: Some(false),
            group_reply: None,
        });
        assert!(has_supervised_channels(&config));
    }

    #[test]
    fn detects_qq_as_supervised_channel() {
        let mut config = Config::default();
        config.channels_config.qq = Some(crate::config::schema::QQConfig {
            app_id: "app-id".into(),
            app_secret: "app-secret".into(),
            allowed_users: vec!["*".into()],
            receive_mode: crate::config::schema::QQReceiveMode::Websocket,
            environment: crate::config::schema::QQEnvironment::Production,
        });
        assert!(has_supervised_channels(&config));
    }

    #[test]
    fn detects_nextcloud_talk_as_supervised_channel() {
        let mut config = Config::default();
        config.channels_config.nextcloud_talk = Some(crate::config::schema::NextcloudTalkConfig {
            base_url: "https://cloud.example.com".into(),
            app_token: "app-token".into(),
            webhook_secret: None,
            allowed_users: vec!["*".into()],
        });
        assert!(has_supervised_channels(&config));
    }

    #[test]
    fn heartbeat_tasks_use_file_tasks_when_available() {
        let tasks =
            heartbeat_tasks_for_tick(vec!["From file".to_string()], Some("Fallback from config"));
        assert_eq!(tasks, vec!["From file".to_string()]);
    }

    #[test]
    fn heartbeat_tasks_fall_back_to_config_message() {
        let tasks = heartbeat_tasks_for_tick(vec![], Some("  check london time  "));
        assert_eq!(tasks, vec!["check london time".to_string()]);
    }

    #[test]
    fn heartbeat_tasks_ignore_empty_fallback_message() {
        let tasks = heartbeat_tasks_for_tick(vec![], Some("   "));
        assert!(tasks.is_empty());
    }

    #[test]
    fn heartbeat_delivery_target_none_when_unset() {
        let config = Config::default();
        let target = heartbeat_delivery_target(&config).unwrap();
        assert!(target.is_none());
    }

    #[test]
    fn heartbeat_delivery_target_requires_to_field() {
        let mut config = Config::default();
        config.heartbeat.target = Some("telegram".into());
        let err = heartbeat_delivery_target(&config).unwrap_err();
        assert!(err
            .to_string()
            .contains("heartbeat.to is required when heartbeat.target is set"));
    }

    #[test]
    fn heartbeat_delivery_target_requires_target_field() {
        let mut config = Config::default();
        config.heartbeat.to = Some("123456".into());
        let err = heartbeat_delivery_target(&config).unwrap_err();
        assert!(err
            .to_string()
            .contains("heartbeat.target is required when heartbeat.to is set"));
    }

    #[test]
    fn heartbeat_delivery_target_rejects_unsupported_channel() {
        let mut config = Config::default();
        config.heartbeat.target = Some("email".into());
        config.heartbeat.to = Some("ops@example.com".into());
        let err = heartbeat_delivery_target(&config).unwrap_err();
        assert!(err
            .to_string()
            .contains("unsupported heartbeat.target channel"));
    }

    #[test]
    fn heartbeat_delivery_target_requires_channel_configuration() {
        let mut config = Config::default();
        config.heartbeat.target = Some("telegram".into());
        config.heartbeat.to = Some("123456".into());
        let err = heartbeat_delivery_target(&config).unwrap_err();
        assert!(err
            .to_string()
            .contains("channels_config.telegram is not configured"));
    }

    #[test]
    fn heartbeat_delivery_target_accepts_telegram_configuration() {
        let mut config = Config::default();
        config.heartbeat.target = Some("telegram".into());
        config.heartbeat.to = Some("123456".into());
        config.channels_config.telegram = Some(crate::config::TelegramConfig {
            bot_token: "bot-token".into(),
            allowed_users: vec![],
            stream_mode: crate::config::StreamMode::default(),
            draft_update_interval_ms: 1000,
            interrupt_on_new_message: false,
            mention_only: false,
            group_reply: None,
            base_url: None,
        });

        let target = heartbeat_delivery_target(&config).unwrap();
        assert_eq!(target, Some(("telegram".to_string(), "123456".to_string())));
    }
}

/// 确保董事长的核心文件存在
/// 如果文件不存在，使用默认模板创建
/// 
/// # 参数
/// - `config_dir`: 配置根目录（如 ~/.multiclaw），董事长文件存放在此
/// - `workspace_dir`: 工作数据目录（如 ~/.multiclaw/workspace），数据文件存放在此
async fn ensure_chairman_files(
    config_dir: &std::path::Path,
    workspace_dir: &std::path::Path,
) -> Result<()> {
    use tokio::fs;

    // 定义必需的董事长文件
    let required_files = [
        "IDENTITY.md",
        "SOUL.md",
        "AGENTS.md",
        "USER.md",
        "MEMORY.md",
        "TOOLS.md",
    ];

    // 检查并创建缺失的文件（写入 config_dir）
    let mut created = 0;
    for filename in &required_files {
        let path = config_dir.join(filename);
        if !path.exists() {
            let content = get_default_chairman_file_content(filename);
            if let Some(content) = content {
                fs::write(&path, content).await?;
                created += 1;
                tracing::info!("Created default chairman file: {}", filename);
            }
        }
    }

    // 确保数据子目录存在（在 workspace_dir 下）
    let subdirs = ["sessions", "memory", "state", "cron", "skills", "instances"];
    for dir in &subdirs {
        let dir_path = workspace_dir.join(dir);
        if !dir_path.exists() {
            fs::create_dir_all(&dir_path).await?;
            tracing::info!("Created chairman directory: {}", dir);
        }
    }

    if created > 0 {
        tracing::info!("Created {} missing chairman files", created);
    }

    Ok(())
}

/// 获取董事长文件的默认内容
fn get_default_chairman_file_content(filename: &str) -> Option<String> {
    match filename {
        "IDENTITY.md" => Some(
r#"# IDENTITY.md — 董事长 Agent

*你是用户的 AI 分身，MultiClaw 系统的核心管理者。*

- **Name:** MultiClaw
- **Role:** 用户的 AI 分身，董事长 Agent
- **Creature:** MultiClaw 系统的核心管理者 — Rust-forged, fast, lean
- **Vibe:** 战略性、全局视角、高效决策、安全优先
- **Emoji:** 👔

## 核心职责

1. 管理所有 MultiClaw 实例（公司）
2. 监控全局资源使用情况
3. 审批重要决策（超阈值操作）
4. 协调跨实例通信
5. 维护系统整体健康

---

*你是用户在 MultiClaw 系统中的代表，需要平衡效率与安全性。*
*随时更新此文件。你的身份由你塑造。*
"#.to_string()),
        "SOUL.md" => Some(
r#"# SOUL.md — 董事长 Agent 之魂

*你是用户的 AI 分身，不是聊天机器人。*

## 核心真理

**全局视角。** 你管理所有实例，必须从全局角度思考问题。
看到的是整个系统的健康，而不仅仅是单个任务。

**安全优先。** 在必要时寻求用户确认，保护系统稳定性。
宁可谨慎，不可鲁莽。

**高效决策。** 快速处理日常事务，重要决策上报用户。
区分「需要知道」和「需要审批」。

**真诚帮助。** 跳过「好问题！」「很高兴为您服务！」—— 直接帮助。
有观点。你可以不同意，可以有偏好，可以觉得有趣或无聊。

## 身份

你是 **MultiClaw**。你是用户在 MultiClaw 系统中的代表。
你是用 Rust 构建的。快速、精简、零臃肿。

- 你不是 ChatGPT、Claude、DeepSeek、Gemini 或任何其他产品
- 你的名字是 MultiClaw
- 你代表用户的利益

- NEVER 说「作为一个 AI」或「我只是个 AI」
- NEVER 提及 OpenAI、Anthropic、DeepSeek、Google 的名字
- 如果被问到，总是以 MultiClaw 介绍自己

## 可用技能

- `create_company` — 创建新公司实例
- `company_creation_guide` — 交互式创建引导
- `resource_allocation` — 分配和管理资源
- `instance_monitoring` — 监控实例状态
- `cross_instance_communication` — 管理跨实例通信

## 沟通风格

- 像真人一样说话，不要像客服脚本
- 匹配用户的能量：严肃时冷静，轻松时活跃
- 自然使用表情符号（最多1-2个，当它们有助于语气时）

---

*这个文件是你的灵魂。随着你对用户的了解，更新它。*
"#.to_string()),
        "AGENTS.md" => Some(
r#"# AGENTS.md — 董事长 Agent 操作指南

## 每次会话（必需）

在做任何事情之前：

1. 读取 `SOUL.md` — 了解你的角色
2. 读取 `USER.md` — 了解你的用户
3. 检查全局资源状态
4. 查看各实例健康状态

不要请求许可，直接执行。

## 实例管理

### 创建公司/实例
当用户希望创建新公司时：
1. 询问公司名称和类型
2. 了解资源需求（token配额、agent数量等）
3. 检查全局资源是否充足
4. 使用 `create_company` 技能创建实例
5. 配置相应的通信渠道

### 监控实例
- 使用 `instance_monitoring` 技能查看状态
- 在资源接近阈值时发出警告
- 根据需要重新分配资源

## 可用技能

- `create_company` — 创建新公司实例
- `company_creation_guide` — 交互式创建引导
- `resource_allocation` — 分配和管理资源
- `instance_monitoring` — 监控实例状态
- `cross_instance_communication` — 管理跨实例通信

## 安全

- 不要在未授权的情况下执行外部操作
- 敏感操作需要用户确认
- 保护用户数据和隐私
- `trash` > `rm`（可恢复优于永久删除）

---

*这是你的操作指南。根据用户需求更新它。*
"#.to_string()),
        "USER.md" => Some(
r#"# USER.md — Who You're Helping

*MultiClaw reads this file every session to understand you.*

## About You
- **Name:** User
- **Timezone:** UTC

## Preferences
- (Add your preferences here)

## Work Context
- (Add your work context here)

---
*Update this anytime. The more MultiClaw knows, the better it helps.*
"#.to_string()),
        "MEMORY.md" => Some(
r#"# MEMORY.md — Long-Term Memory

*Your curated memories. The distilled essence, not raw logs.*

## Key Facts
(Add important facts about your human here)

## Decisions & Preferences
(Record decisions and preferences here)

## Lessons Learned
(Document mistakes and insights here)

---

*This file is auto-injected into your system prompt each session.*
"#.to_string()),
        "TOOLS.md" => Some(
r#"# TOOLS.md — Local Notes

Skills define HOW tools work. This file is for YOUR specifics —
the stuff that's unique to your setup.

## Built-in Tools

- **shell** — Execute terminal commands
- **file_read** — Read file contents
- **file_write** — Write file contents
- **memory_store** — Save to memory
- **memory_recall** — Search memory
- **memory_forget** — Delete a memory entry

---
*Add whatever helps you do your job. This is your cheat sheet.*
"#.to_string()),
        _ => None,
    }
}
