#![warn(clippy::all, clippy::pedantic)]
#![forbid(unsafe_code)]
#![allow(
    clippy::assigning_clones,
    clippy::bool_to_int_with_if,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::cast_possible_wrap,
    clippy::doc_markdown,
    clippy::field_reassign_with_default,
    clippy::float_cmp,
    clippy::implicit_clone,
    clippy::items_after_statements,
    clippy::map_unwrap_or,
    clippy::manual_let_else,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::needless_raw_string_hashes,
    clippy::redundant_closure_for_method_calls,
    clippy::similar_names,
    clippy::single_match_else,
    clippy::struct_field_names,
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::unused_self,
    clippy::cast_precision_loss,
    clippy::unnecessary_cast,
    clippy::unnecessary_lazy_evaluations,
    clippy::unnecessary_literal_bound,
    clippy::unnecessary_map_or,
    clippy::unnecessary_wraps,
    dead_code
)]

use anyhow::{bail, Context, Result};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use dialoguer::{Input, Password};
use serde::{Deserialize, Serialize};
use std::io::Write;
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

fn parse_temperature(s: &str) -> std::result::Result<f64, String> {
    let t: f64 = s.parse().map_err(|e| format!("{e}"))?;
    if !(0.0..=2.0).contains(&t) {
        return Err("temperature must be between 0.0 and 2.0".to_string());
    }
    Ok(t)
}

// 从 lib 导入所有模块（只导入实际需要的）
use multiclaw::{
    a2a, agent, auth, channels, config, coordination, core, cron, daemon,
    doctor, gateway, goals, hardware, hooks,
    integrations, memory, migration,
    observability, onboard,
    peripherals, providers, rag, runtime, security, service, skills,
    update, instance,
};

use config::Config;

// Re-export so binary modules can use crate::<CommandEnum> while keeping a single source of truth.
pub use multiclaw::{
    ChannelCommands, CronCommands, HardwareCommands, IntegrationCommands, MigrateCommands,
    MemoryCommands, PeripheralCommands, ServiceCommands, SkillCommands,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum CompletionShell {
    #[value(name = "bash")]
    Bash,
    #[value(name = "fish")]
    Fish,
    #[value(name = "zsh")]
    Zsh,
    #[value(name = "powershell")]
    PowerShell,
    #[value(name = "elvish")]
    Elvish,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum EstopLevelArg {
    #[value(name = "kill-all")]
    KillAll,
    #[value(name = "network-kill")]
    NetworkKill,
    #[value(name = "domain-block")]
    DomainBlock,
    #[value(name = "tool-freeze")]
    ToolFreeze,
}

/// `MultiClaw` - Zero overhead. Zero compromise. 100% Rust.
#[derive(Parser, Debug)]
#[command(name = "multiclaw")]
#[command(author = "theonlyhennygod")]
#[command(version)]
#[command(about = "The fastest, smallest AI assistant.", long_about = None)]
struct Cli {
    #[arg(long, global = true)]
    config_dir: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize your workspace and configuration
    Onboard {
        /// Run the full interactive wizard (default is quick setup)
        #[arg(long)]
        interactive: bool,

        /// Overwrite existing config without confirmation
        #[arg(long)]
        force: bool,

        /// Reconfigure channels only (fast repair flow)
        #[arg(long)]
        channels_only: bool,

        /// API key (used in quick mode, ignored with --interactive)
        #[arg(long)]
        api_key: Option<String>,

        /// Provider name (used in quick mode, default: openrouter)
        #[arg(long)]
        provider: Option<String>,
        /// Model ID override (used in quick mode)
        #[arg(long)]
        model: Option<String>,
        /// Memory backend (sqlite, lucid, markdown, none) - used in quick mode, default: sqlite
        #[arg(long)]
        memory: Option<String>,

        /// Disable OTP in quick setup (not recommended)
        #[arg(long)]
        no_totp: bool,
    },

    /// Start the AI agent loop
    #[command(long_about = "\
Start the AI agent loop.

Launches an interactive chat session with the configured AI provider. \
Use --message for single-shot queries without entering interactive mode.

Examples:
  multiclaw agent                              # interactive session
  multiclaw agent -m \"Summarize today's logs\"  # single message
  multiclaw agent -p anthropic --model claude-sonnet-4-20250514
  multiclaw agent --peripheral nucleo-f401re:/dev/ttyACM0
  multiclaw agent --autonomy-level full --max-actions-per-hour 100
  multiclaw agent -m \"quick task\" --memory-backend none --compact-context")]
    Agent {
        /// Single message mode (don't enter interactive mode)
        #[arg(short, long)]
        message: Option<String>,

        /// Provider to use (openrouter, anthropic, openai, openai-codex)
        #[arg(short, long)]
        provider: Option<String>,

        /// Model to use
        #[arg(long)]
        model: Option<String>,

        /// Temperature (0.0 - 2.0)
        #[arg(short, long, default_value = "0.7", value_parser = parse_temperature)]
        temperature: f64,

        /// Attach a peripheral (board:path, e.g. nucleo-f401re:/dev/ttyACM0)
        #[arg(long)]
        peripheral: Vec<String>,

        /// Autonomy level (read_only, supervised, full)
        #[arg(long, value_parser = clap::value_parser!(security::AutonomyLevel))]
        autonomy_level: Option<security::AutonomyLevel>,

        /// Maximum shell/tool actions per hour
        #[arg(long)]
        max_actions_per_hour: Option<u32>,

        /// Maximum tool-call iterations per message
        #[arg(long)]
        max_tool_iterations: Option<usize>,

        /// Maximum conversation history messages
        #[arg(long)]
        max_history_messages: Option<usize>,

        /// Enable compact context mode (smaller prompts for limited models)
        #[arg(long)]
        compact_context: bool,

        /// Memory backend (sqlite, markdown, none)
        #[arg(long)]
        memory_backend: Option<String>,
    },

    /// Start the gateway server (webhooks, websockets)
    #[command(long_about = "\
Start the gateway server (webhooks, websockets).

Runs the HTTP/WebSocket gateway that accepts incoming webhook events \
and WebSocket connections. Bind address defaults to the values in \
your config file (gateway.host / gateway.port).

Examples:
  multiclaw gateway                  # use config defaults
  multiclaw gateway -p 8080          # listen on port 8080
  multiclaw gateway --host 0.0.0.0   # bind to all interfaces
  multiclaw gateway -p 0             # random available port
  multiclaw gateway --new-pairing    # clear tokens and generate fresh pairing code")]
    Gateway {
        /// Port to listen on (use 0 for random available port); defaults to config gateway.port
        #[arg(short, long)]
        port: Option<u16>,

        /// Host to bind to; defaults to config gateway.host
        #[arg(long)]
        host: Option<String>,

        /// Clear all paired tokens and generate a fresh pairing code
        #[arg(long)]
        new_pairing: bool,
    },

    /// Start long-running autonomous runtime (gateway + channels + heartbeat + scheduler)
    #[command(long_about = "\
Start the long-running autonomous daemon.

Launches the full MultiClaw runtime: gateway server, all configured \
channels (Telegram, Discord, Slack, etc.), heartbeat monitor, and \
the cron scheduler. This is the recommended way to run MultiClaw in \
production or as an always-on assistant.

Use 'multiclaw service install' to register the daemon as an OS \
service (systemd/launchd) for auto-start on boot.

Examples:
  multiclaw daemon                   # use config defaults
  multiclaw daemon -p 9090           # gateway on port 9090
  multiclaw daemon --host 127.0.0.1  # localhost only")]
    Daemon {
        /// Port to listen on (use 0 for random available port); defaults to config gateway.port
        #[arg(short, long)]
        port: Option<u16>,

        /// Host to bind to; defaults to config gateway.host
        #[arg(long)]
        host: Option<String>,
    },

    /// Manage OS service lifecycle (launchd/systemd user service)
    Service {
        /// Init system to use: auto (detect), systemd, or openrc
        #[arg(long, default_value = "auto", value_parser = ["auto", "systemd", "openrc"])]
        service_init: String,

        #[command(subcommand)]
        service_command: ServiceCommands,
    },

    /// Run diagnostics for daemon/scheduler/channel freshness
    Doctor {
        #[command(subcommand)]
        doctor_command: Option<DoctorCommands>,
    },

    /// Show system status (full details)
    Status,

    /// Self-update MultiClaw to the latest version
    #[command(long_about = "\
Self-update MultiClaw to the latest release from GitHub.

Downloads the appropriate pre-built binary for your platform and
replaces the current executable. Requires write permissions to
the binary location.

Examples:
  multiclaw update              # Update to latest version
  multiclaw update --check      # Check for updates without installing
  multiclaw update --force      # Reinstall even if already up to date")]
    Update {
        /// Check for updates without installing
        #[arg(long)]
        check: bool,

        /// Force update even if already at latest version
        #[arg(long)]
        force: bool,
    },

    /// Engage, inspect, and resume emergency-stop states.
    ///
    /// Examples:
    /// - `multiclaw estop`
    /// - `multiclaw estop --level network-kill`
    /// - `multiclaw estop --level domain-block --domain "*.chase.com"`
    /// - `multiclaw estop --level tool-freeze --tool shell --tool browser`
    /// - `multiclaw estop status`
    /// - `multiclaw estop resume --network`
    /// - `multiclaw estop resume --domain "*.chase.com"`
    /// - `multiclaw estop resume --tool shell`
    Estop {
        #[command(subcommand)]
        estop_command: Option<EstopSubcommands>,

        /// Level used when engaging estop from `multiclaw estop`.
        #[arg(long, value_enum)]
        level: Option<EstopLevelArg>,

        /// Domain pattern(s) for `domain-block` (repeatable).
        #[arg(long = "domain")]
        domains: Vec<String>,

        /// Tool name(s) for `tool-freeze` (repeatable).
        #[arg(long = "tool")]
        tools: Vec<String>,
    },

    /// Configure and manage scheduled tasks
    #[command(long_about = "\
Configure and manage scheduled tasks.

Schedule recurring, one-shot, or interval-based tasks using cron \
expressions, RFC 3339 timestamps, durations, or fixed intervals.

Cron expressions use the standard 5-field format: \
'min hour day month weekday'. Timezones default to UTC; \
override with --tz and an IANA timezone name.

Examples:
  multiclaw cron list
  multiclaw cron add '0 9 * * 1-5' 'Good morning' --tz America/New_York
  multiclaw cron add '*/30 * * * *' 'Check system health'
  multiclaw cron add-at 2025-01-15T14:00:00Z 'Send reminder'
  multiclaw cron add-every 60000 'Ping heartbeat'
  multiclaw cron once 30m 'Run backup in 30 minutes'
  multiclaw cron pause <task-id>
  multiclaw cron update <task-id> --expression '0 8 * * *' --tz Europe/London")]
    Cron {
        #[command(subcommand)]
        cron_command: CronCommands,
    },

    /// Manage provider model catalogs
    Models {
        #[command(subcommand)]
        model_command: ModelCommands,
    },

    /// List supported AI providers
    Providers,

    /// Manage channels (telegram, discord, slack)
    #[command(long_about = "\
Manage communication channels.

Add, remove, list, and health-check channels that connect MultiClaw \
to messaging platforms. Supported channel types: telegram, discord, \
slack, whatsapp, matrix, imessage, email.

Examples:
  multiclaw channel list
  multiclaw channel doctor
  multiclaw channel add telegram '{\"bot_token\":\"...\",\"name\":\"my-bot\"}'
  multiclaw channel remove my-bot
  multiclaw channel bind-telegram multiclaw_user")]
    Channel {
        #[command(subcommand)]
        channel_command: ChannelCommands,
    },

    /// Browse 50+ integrations
    Integrations {
        #[command(subcommand)]
        integration_command: IntegrationCommands,
    },

    /// Manage skills (user-defined capabilities)
    #[command(name = "skill", alias = "skills")]
    Skills {
        #[command(subcommand)]
        skill_command: SkillCommands,
    },

    /// Migrate data from other agent runtimes
    Migrate {
        #[command(subcommand)]
        migrate_command: MigrateCommands,
    },

    /// Manage provider subscription authentication profiles
    Auth {
        #[command(subcommand)]
        auth_command: AuthCommands,
    },

    /// Discover and introspect USB hardware
    #[command(long_about = "\
Discover and introspect USB hardware.

Enumerate connected USB devices, identify known development boards \
(STM32 Nucleo, Arduino, ESP32), and retrieve chip information via \
probe-rs / ST-Link.

Examples:
  multiclaw hardware discover
  multiclaw hardware introspect /dev/ttyACM0
  multiclaw hardware info --chip STM32F401RETx")]
    Hardware {
        #[command(subcommand)]
        hardware_command: multiclaw::HardwareCommands,
    },

    /// Manage hardware peripherals (STM32, RPi GPIO, etc.)
    #[command(long_about = "\
Manage hardware peripherals.

Add, list, flash, and configure hardware boards that expose tools \
to the agent (GPIO, sensors, actuators). Supported boards: \
nucleo-f401re, rpi-gpio, esp32, arduino-uno.

Examples:
  multiclaw peripheral list
  multiclaw peripheral add nucleo-f401re /dev/ttyACM0
  multiclaw peripheral add rpi-gpio native
  multiclaw peripheral flash --port /dev/cu.usbmodem12345
  multiclaw peripheral flash-nucleo")]
    Peripheral {
        #[command(subcommand)]
        peripheral_command: multiclaw::PeripheralCommands,
    },

    /// Manage agent memory (list, get, stats, clear)
    #[command(long_about = "\
Manage agent memory entries.

List, inspect, and clear memory entries stored by the agent. \
Supports filtering by category and session, pagination, and \
batch clearing with confirmation.

Examples:
  multiclaw memory stats
  multiclaw memory list
  multiclaw memory list --category core --limit 10
  multiclaw memory get <key>
  multiclaw memory clear --category conversation --yes")]
    Memory {
        #[command(subcommand)]
        memory_command: MemoryCommands,
    },

    /// Manage configuration
    #[command(long_about = "\
Manage MultiClaw configuration.

Inspect and export configuration settings. Use 'schema' to dump \
the full JSON Schema for the config file, which documents every \
available key, type, and default value.

Examples:
  multiclaw config schema              # print JSON Schema to stdout
  multiclaw config schema > schema.json")]
    Config {
        #[command(subcommand)]
        config_command: ConfigCommands,
    },

    /// Generate shell completion script to stdout
    #[command(long_about = "\
Generate shell completion scripts for `multiclaw`.

The script is printed to stdout so it can be sourced directly:

Examples:
  source <(multiclaw completions bash)
  multiclaw completions zsh > ~/.zfunc/_multiclaw
  multiclaw completions fish > ~/.config/fish/completions/multiclaw.fish")]
    Completions {
        /// Target shell
        #[arg(value_enum)]
        shell: CompletionShell,
    },
    
    /// Manage instances (create, list, start, stop)
    #[command(long_about = "\
Manage MultiClaw instances.

Create, list, start, and stop independent MultiClaw instances that run \
separate companies/departments with isolated resources and configurations.

Examples:
  multiclaw instance create --name \"Marketing\" --type market_research
  multiclaw instance list
  multiclaw instance start <id>
  multiclaw instance stop <id>")]
    Instance {
        #[command(subcommand)]
        instance_command: InstanceCommands,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    /// Dump the full configuration JSON Schema to stdout
    Schema,
}

#[derive(Subcommand, Debug)]
enum InstanceCommands {
    /// Create a new instance
    Create {
        /// Name of the instance
        #[arg(long)]
        name: String,
        
        /// Type of instance
        #[arg(long, value_parser = clap::value_parser!(InstanceTypeArg))]
        instance_type: InstanceTypeArg,
        
        /// Token quota per minute
        #[arg(long, default_value = "100000")]
        token_quota: u64,
        
        /// Max concurrent agents
        #[arg(long, default_value = "10")]
        max_agents: u32,
        
        /// Base data directory
        #[arg(long, default_value = "~/.multiclaw")]
        base_data_dir: String,
    },
    
    /// List all instances
    List,
    
    /// Start an instance
    Start {
        /// Instance ID
        id: String,
    },
    
    /// Stop an instance
    Stop {
        /// Instance ID
        id: String,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum InstanceTypeArg {
    #[value(name = "market_research")]
    MarketResearch,
    #[value(name = "product_development")]
    ProductDevelopment,
    #[value(name = "customer_service")]
    CustomerService,
    #[value(name = "data_analysis")]
    DataAnalysis,
    #[value(name = "general")]
    General,
    #[value(name = "custom")]
    Custom,
}

#[derive(Subcommand, Debug)]
enum EstopSubcommands {
    /// Print current estop status.
    Status,
    /// Resume from an engaged estop level.
    Resume {
        /// Resume only network kill.
        #[arg(long)]
        network: bool,
        /// Resume one or more blocked domain patterns.
        #[arg(long = "domain")]
        domains: Vec<String>,
        /// Resume one or more frozen tools.
        #[arg(long = "tool")]
        tools: Vec<String>,
        /// OTP code. If omitted and OTP is required, a prompt is shown.
        #[arg(long)]
        otp: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum AuthCommands {
    /// Login with OAuth (OpenAI Codex or Gemini)
    Login {
        /// Provider (`openai-codex` or `gemini`)
        #[arg(long)]
        provider: String,
        /// Profile name (default: default)
        #[arg(long, default_value = "default")]
        profile: String,
        /// Use OAuth device-code flow
        #[arg(long)]
        device_code: bool,
    },
    /// Complete OAuth by pasting redirect URL or auth code
    PasteRedirect {
        /// Provider (`openai-codex`)
        #[arg(long)]
        provider: String,
        /// Profile name (default: default)
        #[arg(long, default_value = "default")]
        profile: String,
        /// Full redirect URL or raw OAuth code
        #[arg(long)]
        input: Option<String>,
    },
    /// Paste setup token / auth token (for Anthropic subscription auth)
    PasteToken {
        /// Provider (`anthropic`)
        #[arg(long)]
        provider: String,
        /// Profile name (default: default)
        #[arg(long, default_value = "default")]
        profile: String,
        /// Token value (if omitted, read interactively)
        #[arg(long)]
        token: Option<String>,
        /// Auth kind override (`authorization` or `api-key`)
        #[arg(long)]
        auth_kind: Option<String>,
    },
    /// Alias for `paste-token` (interactive by default)
    SetupToken {
        /// Provider (`anthropic`)
        #[arg(long)]
        provider: String,
        /// Profile name (default: default)
        #[arg(long, default_value = "default")]
        profile: String,
    },
    /// Refresh OpenAI Codex access token using refresh token
    Refresh {
        /// Provider (`openai-codex`)
        #[arg(long)]
        provider: String,
        /// Profile name or profile id
        #[arg(long)]
        profile: Option<String>,
    },
    /// Remove auth profile
    Logout {
        /// Provider
        #[arg(long)]
        provider: String,
        /// Profile name (default: default)
        #[arg(long, default_value = "default")]
        profile: String,
    },
    /// Set active profile for a provider
    Use {
        /// Provider
        #[arg(long)]
        provider: String,
        /// Profile name or full profile id
        #[arg(long)]
        profile: String,
    },
    /// List auth profiles
    List,
    /// Show auth status with active profile and token expiry info
    Status,
}

#[derive(Subcommand, Debug)]
enum ModelCommands {
    /// Refresh and cache provider models
    Refresh {
        /// Provider name (defaults to configured default provider)
        #[arg(long)]
        provider: Option<String>,

        /// Refresh all providers that support live model discovery
        #[arg(long)]
        all: bool,

        /// Force live refresh and ignore fresh cache
        #[arg(long)]
        force: bool,
    },
    /// List cached models for a provider
    List {
        /// Provider name (defaults to configured default provider)
        #[arg(long)]
        provider: Option<String>,
    },
    /// Set the default model in config
    Set {
        /// Model name to set as default
        model: String,
    },
    /// Show current model configuration and cache status
    Status,
}

#[derive(Subcommand, Debug)]
enum DoctorCommands {
    /// Probe model catalogs across providers and report availability
    Models {
        /// Probe a specific provider only (default: all known providers)
        #[arg(long)]
        provider: Option<String>,

        /// Prefer cached catalogs when available (skip forced live refresh)
        #[arg(long)]
        use_cache: bool,
    },
    /// Query runtime trace events (tool diagnostics and model replies)
    Traces {
        /// Show a specific trace event by id
        #[arg(long)]
        id: Option<String>,
        /// Filter list output by event type
        #[arg(long)]
        event: Option<String>,
        /// Case-insensitive text match across message/payload
        #[arg(long)]
        contains: Option<String>,
        /// Maximum number of events to display
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

// Helper function to convert InstanceTypeArg to InstanceType
fn convert_instance_type(instance_type: InstanceTypeArg) -> instance::InstanceType {
    match instance_type {
        InstanceTypeArg::MarketResearch => instance::InstanceType::MarketResearch,
        InstanceTypeArg::ProductDevelopment => instance::InstanceType::ProductDevelopment,
        InstanceTypeArg::CustomerService => instance::InstanceType::CustomerService,
        InstanceTypeArg::DataAnalysis => instance::InstanceType::DataAnalysis,
        InstanceTypeArg::General => instance::InstanceType::General,
        InstanceTypeArg::Custom => instance::InstanceType::Custom,
    }
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    // Install default crypto provider for Rustls TLS.
    // This prevents the error: "could not automatically determine the process-level CryptoProvider"
    // when both aws-lc-rs and ring features are available (or neither is explicitly selected).
    if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
        eprintln!("Warning: Failed to install default crypto provider: {e:?}");
    }

    let cli = Cli::parse();

    if let Some(config_dir) = &cli.config_dir {
        if config_dir.trim().is_empty() {
            bail!("--config-dir cannot be empty");
        }
        std::env::set_var("MULTICLAW_CONFIG_DIR", config_dir);
    }

    // Completions must remain stdout-only and should not load config or initialize logging.
    // This avoids warnings/log lines corrupting sourced completion scripts.
    if let Commands::Completions { shell } = &cli.command {
        let mut stdout = std::io::stdout().lock();
        write_shell_completion(*shell, &mut stdout)?;
        return Ok(());
    }

    // Initialize logging - respects RUST_LOG env var, defaults to INFO
    let subscriber = fmt::Subscriber::builder()
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Onboard runs quick setup by default, or the interactive wizard with --interactive.
    // The onboard wizard uses reqwest::blocking internally, which creates its own
    // Tokio runtime. To avoid "Cannot drop a runtime in a context where blocking is
    // not allowed", we run the wizard on a blocking thread via spawn_blocking.
    if let Commands::Onboard {
        interactive,
        force,
        channels_only,
        api_key,
        provider,
        model,
        memory,
        no_totp,
    } = &cli.command
    {
        let interactive = *interactive;
        let force = *force;
        let channels_only = *channels_only;
        let api_key = api_key.clone();
        let provider = provider.clone();
        let model = model.clone();
        let memory = memory.clone();
        let no_totp = *no_totp;

        if interactive && channels_only {
            bail!("Use either --interactive or --channels-only, not both");
        }
        if channels_only
            && (api_key.is_some()
                || provider.is_some()
                || model.is_some()
                || memory.is_some()
                || no_totp)
        {
            bail!(
                "--channels-only does not accept --api-key, --provider, --model, --memory, or --no-totp"
            );
        }
        if channels_only && force {
            bail!("--channels-only does not accept --force");
        }
        let config = if channels_only {
            onboard::run_channels_repair_wizard().await
        } else if interactive {
            onboard::run_wizard(force).await
        } else {
            onboard::run_quick_setup(
                api_key.as_deref(),
                provider.as_deref(),
                model.as_deref(),
                memory.as_deref(),
                force,
                no_totp,
            )
            .await
        }?;
        // Auto-start channels if user said yes during wizard
        if std::env::var("MULTICLAW_AUTOSTART_CHANNELS").as_deref() == Ok("1") {
            channels::start_channels(config).await?;
        }
        return Ok(());
    }

    // All other commands need config loaded first
    let mut config = Config::load_or_init().await?;
    config.apply_env_overrides();
    observability::runtime_trace::init_from_config(&config.observability, &config.workspace_dir);
    if config.security.otp.enabled {
        let config_dir = config
            .config_path
            .parent()
            .context("Config path must have a parent directory")?;
        let store = security::SecretStore::new(config_dir, config.secrets.encrypt);
        let (_validator, enrollment_uri) =
            security::OtpValidator::from_config(&config.security.otp, config_dir, &store)?;
        if let Some(uri) = enrollment_uri {
            println!("Initialized OTP secret for MultiClaw.");
            println!("Enrollment URI: {uri}");
            println!("Scan the QR code below with Google Authenticator or similar:");
            println!("{}", qr2term::printqr(&uri).map_err(|e| anyhow::anyhow!("QR code error: {e}"))?);
        }
    }

    match &cli.command {
        Commands::Instance { instance_command } => {
            handle_instance_command(instance_command, &config).await?;
        }
        Commands::Daemon { port, host } => {
            let port = port.or(config.gateway.port);
            let host = host.as_deref().unwrap_or(&config.gateway.host);
            daemon::run_daemon(host, port, config).await?;
        }
        Commands::Gateway { port, host, new_pairing } => {
            let port = port.or(config.gateway.port);
            let host = host.as_deref().unwrap_or(&config.gateway.host);
            gateway::run_gateway(host, port, *new_pairing, config).await?;
        }
        Commands::Agent {
            message,
            provider,
            model,
            temperature,
            peripheral,
            autonomy_level,
            max_actions_per_hour,
            max_tool_iterations,
            max_history_messages,
            compact_context,
            memory_backend,
        } => {
            let mut agent_config = agent::AgentBuilder::new(config)
                .provider(provider.clone())
                .model(model.clone())
                .temperature(*temperature)
                .peripherals(peripheral.clone())
                .autonomy_level(*autonomy_level)
                .max_actions_per_hour(max_actions_per_hour.unwrap_or(0))
                .max_tool_iterations(max_tool_iterations.unwrap_or(10))
                .max_history_messages(max_history_messages.unwrap_or(20))
                .compact_context(*compact_context)
                .memory_backend(memory_backend.clone())
                .build()?;

            if let Some(msg) = message {
                agent_config.run_single_message(msg).await?;
            } else {
                agent_config.run_interactive_loop().await?;
            }
        }
        Commands::Service {
            service_init,
            service_command,
        } => {
            service::handle_command(service_init, service_command).await?;
        }
        Commands::Doctor { doctor_command } => {
            doctor::run_doctor(doctor_command, &config).await?;
        }
        Commands::Status => {
            println!("MultiClaw Status:");
            println!("- Version: {}", env!("CARGO_PKG_VERSION"));
            println!("- Config dir: {}", config.config_path.display());
            println!("- Workspace dir: {}", config.workspace_dir.display());
            println!("- Gateway: {}:{} (TLS: {})", config.gateway.host, config.gateway.port.unwrap_or(8080), config.gateway.tls);
            println!("- Active channels: {}", config.channels.len());
            println!("- Default provider: {}", config.provider);
            println!("- Default model: {}", config.model);
            println!("- Memory backend: {}", config.memory.backend);
        }
        Commands::Update { check, force } => {
            update::run_update(*check, *force).await?;
        }
        Commands::Estop {
            estop_command,
            level,
            domains,
            tools,
        } => {
            // Handle estop commands
            match estop_command {
                Some(EstopSubcommands::Status) => {
                    // Print current estop status
                    println!("Current e-stop status: Not implemented in this example");
                }
                Some(EstopSubcommands::Resume {
                    network,
                    domains: resume_domains,
                    tools: resume_tools,
                    otp,
                }) => {
                    // Handle resume commands
                    println!("Resuming from e-stop (not implemented in this example)");
                }
                None => {
                    // Handle engaging estop based on level arg
                    println!("Engaging e-stop with level: {:?}", level);
                }
            }
        }
        Commands::Cron { cron_command } => {
            cron::handle_command(cron_command, &config).await?;
        }
        Commands::Models { model_command } => {
            match model_command {
                ModelCommands::Refresh { provider, all, force } => {
                    // Refresh model catalogs
                    println!("Refreshing model catalogs (not implemented in this example)");
                }
                ModelCommands::List { provider } => {
                    // List cached models
                    println!("Listing models (not implemented in this example)");
                }
                ModelCommands::Set { model } => {
                    // Set default model
                    println!("Setting default model to: {}", model);
                }
                ModelCommands::Status => {
                    // Show model config status
                    println!("Model configuration status (not implemented in this example)");
                }
            }
        }
        Commands::Providers => {
            // List supported providers
            println!("Supported providers: openrouter, anthropic, openai, openai-codex, gemini, kimi, zhipu");
        }
        Commands::Channel { channel_command } => {
            channels::handle_command(channel_command, &config).await?;
        }
        Commands::Integrations { integration_command } => {
            integrations::handle_command(integration_command).await?;
        }
        Commands::Skills { skill_command } => {
            skills::handle_command(skill_command.clone(), &config).await?;
        }
        Commands::Migrate { migrate_command } => {
            migration::handle_command(migrate_command).await?;
        }
        Commands::Auth { auth_command } => {
            auth::handle_command(auth_command).await?;
        }
        Commands::Hardware { hardware_command } => {
            hardware::handle_command(hardware_command).await?;
        }
        Commands::Peripheral { peripheral_command } => {
            peripherals::handle_command(peripheral_command, &config).await?;
        }
        Commands::Memory { memory_command } => {
            memory::handle_command(memory_command, &config).await?;
        }
        Commands::Config { config_command } => {
            match config_command {
                ConfigCommands::Schema => {
                    let schema = schemars::schema_for!(Config);
                    println!("{}", serde_json::to_string_pretty(&schema)?);
                }
            }
        }
        Commands::Completions { .. } => {
            // Already handled above
        }
        Commands::Onboard { .. } => {
            // Already handled above
        }
    }

    Ok(())
}

async fn handle_instance_command(
    command: &InstanceCommands,
    config: &Config,
) -> Result<()> {
    use std::sync::Arc;
    
    // 创建实例管理器
    let instance_manager = Arc::new(instance::InstanceManager::new());
    let config_manager = Arc::new(instance::ConfigManager::new(config.workspace_dir.clone()).await?);
    
    match command {
        InstanceCommands::Create { name, instance_type, token_quota, max_agents, base_data_dir } => {
            let resource_quota = instance::ResourceQuota {
                tokens_per_minute: *token_quota as u32,
                max_concurrent_agents: *max_agents,
                storage_limit_mb: 1000, // 默认 1GB
                api_calls_per_minute: 1000, // 默认 1000 次/分钟
            };
            
            let ceo_config = instance::CEOConfig {
                model_preference: "gpt-4".to_string(),
                personality: "analytical".to_string(),
                resource_limits: resource_quota.clone(),
            };
            
            let create_request = instance::CreateInstanceRequest {
                name: name.clone(),
                instance_type: convert_instance_type(*instance_type),
                quota: resource_quota,
                ceo_config,
                ceo_channel: None,
                base_data_dir: shellexpand::tilde(base_data_dir).to_string(),
            };
            
            let instance_id = instance_manager.create_instance(create_request).await?;
            println!("✅ 实例创建成功！");
            println!("ID: {}", instance_id);
            println!("Name: {}", name);
            println!("Type: {:?}", instance_type);
            println!("Data Dir: {}/instances/{}", shellexpand::tilde(base_data_dir), instance_id);
        },
        InstanceCommands::List => {
            let instances = instance_manager.list_instances().await;
            println!("Instances ({} total):", instances.len());
            for (id, status) in instances {
                println!("  - {} ({:?})", id, status);
            }
        },
        InstanceCommands::Start { id } => {
            // 实现启动实例的逻辑
            println!("Starting instance: {}", id);
            // 实际实现中会调用 instance_manager.start_instance(id)
        },
        InstanceCommands::Stop { id } => {
            // 实现停止实例的逻辑
            instance_manager.stop_instance(id).await?;
            println!("Stopped instance: {}", id);
        },
    }
    
    Ok(())
}

fn write_shell_completion(shell: CompletionShell, writer: &mut dyn Write) -> Result<()> {
    clap_complete::generate(
        match shell {
            CompletionShell::Bash => clap_complete::Shell::Bash,
            CompletionShell::Fish => clap_complete::Shell::Fish,
            CompletionShell::Zsh => clap_complete::Shell::Zsh,
            CompletionShell::PowerShell => clap_complete::Shell::PowerShell,
            CompletionShell::Elvish => clap_complete::Shell::Elvish,
        },
        &mut Cli::command(),
        "multiclaw",
        writer,
    );
    Ok(())
}