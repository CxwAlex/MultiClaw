# MultiClaw 项目上下文

## 项目概述

**MultiClaw** 是一个用 100% Rust 编写的 AI 助手运行时基础设施。它被设计为轻量级、快速启动、低内存占用，可在低端硬件（如 $10 的开发板，<5MB RAM）上运行。

### 核心特性
- **Trait 驱动架构**: 所有子系统（Provider、Channel、Memory、Tool、Runtime 等）都是 trait，可通过配置切换实现
- **安全优先**: 配对机制、沙箱、允许列表、速率限制、加密密钥存储
- **可插拔一切**: 模型提供商、通信渠道、工具、内存后端均可替换
- **五层架构 (v6.0)**: 全局层、编排层、核心层、执行层、可观测层

### 技术栈
- **语言**: Rust (edition 2021, rust-version 1.87)
- **异步运行时**: Tokio
- **HTTP 客户端**: reqwest (rustls-tls)
- **序列化**: serde / serde_json
- **CLI 框架**: clap (derive)
- **数据库**: SQLite (rusqlite) / PostgreSQL (可选)
- **日志**: tracing / tracing-subscriber
- **HTTP 服务**: axum

### 支持的渠道
CLI、Telegram、Discord、Slack、Mattermost、iMessage、Matrix、Signal、WhatsApp、Lark、DingTalk、Email、IRC、Nostr、Webhook

### 支持的提供商
OpenRouter、Anthropic、OpenAI、OpenAI Codex (OAuth)、Gemini、Kimi、Zhipu/GLM、自定义 OpenAI 兼容端点

## v6.0 五层架构

MultiClaw v6.0 实现了全新的五层架构设计：

```
┌─────────────────────────────────────────────────────────────┐
│                    可观测层 (Observability)                  │
│                    五层看板系统、监控告警                      │
├─────────────────────────────────────────────────────────────┤
│                      全局层 (Global)                         │
│              董事长 Agent（用户分身）、实例管理                 │
├─────────────────────────────────────────────────────────────┤
│                    编排层 (Orchestration)                    │
│                Skills 系统、任务编排、执行计划                 │
├─────────────────────────────────────────────────────────────┤
│                      核心层 (Core)                           │
│         MemoryCore、ResourceCore、HealthCore                 │
├─────────────────────────────────────────────────────────────┤
│                     执行层 (Execution)                       │
│                  Agent 执行引擎、工具调用                      │
└─────────────────────────────────────────────────────────────┘
```

### 核心组件

#### A2A 通信协议
- 四级通信（L1: Agent内部, L2: 团队内, L3: 跨团队, L4: 跨实例）
- 消息优先级系统
- 权限验证机制
- 审计日志记录

#### 董事长 Agent（用户分身）
- 统一管理所有 MultiClaw 实例
- 双通道通信（通过董事长或直接联系 CEO）
- 实例生命周期管理
- 资源分配和权限控制

#### MemoryCore（分级记忆系统）
- 四级记忆：全局/集群/团队/本地
- 记忆共享策略
- 访问权限控制
- 记忆检索和查询

#### ResourceCore（资源管理系统）
- 资源配额管理
- 动态资源分配
- 使用量监控
- 资源争用处理

#### HealthCore（健康检查系统）
- 组件健康监控
- 自动故障检测
- 恢复机制

#### Skills 编排系统
- 技能注册和发现
- 执行计划管理
- 资源需求验证
- 执行状态跟踪

## 构建与运行

### 开发构建
```bash
# 开发构建
cargo build

# 发布构建（优化体积）
cargo build --release

# 快速发布构建（高内存机器，并行编译）
cargo build --profile release-fast

# 仅编译库
cargo build --lib

# 仅编译二进制
cargo build --bin multiclaw
```

### 运行
```bash
# 通过 cargo 运行（开发模式）
cargo run --release -- --help
cargo run --release -- status

# 安装后直接运行
multiclaw --help
```

### 测试
```bash
# 运行所有测试
cargo test

# 运行库测试
cargo test --lib

# 运行特定测试
cargo test test_name

# 运行基准测试
cargo bench
```

### Lint 与格式化
```bash
# Clippy 检查（项目使用 pedantic 级别）
cargo clippy --all-targets --all-features

# 格式化检查
cargo fmt --check

# 格式化
cargo fmt
```

### 安全审计
```bash
# 依赖安全检查
cargo deny check
```

## 主要 CLI 命令

| 命令 | 用途 |
|------|------|
| `multiclaw onboard` | 初始化配置（快速模式或交互式向导） |
| `multiclaw agent` | 启动 AI 代理（交互式或单消息模式） |
| `multiclaw gateway` | 启动 Webhook/WebSocket 网关 |
| `multiclaw daemon` | 启动完整自治运行时（网关 + 渠道 + 心跳 + 调度器） |
| `multiclaw status` | 显示系统状态 |
| `multiclaw doctor` | 运行诊断 |
| `multiclaw channel` | 管理通信渠道 |
| `multiclaw providers` | 列出支持的 AI 提供商 |
| `multiclaw models` | 管理提供商模型目录 |
| `multiclaw cron` | 管理定时任务 |
| `multiclaw estop` | 紧急停止管理 |
| `multiclaw service` | 管理 OS 服务（systemd/launchd） |
| `multiclaw hardware` | 发现和检测 USB 硬件 |
| `multiclaw peripheral` | 管理硬件外设 |
| `multiclaw memory` | 管理代理记忆 |
| `multiclaw auth` | 管理订阅认证配置文件 |
| `multiclaw skills` | 管理技能（v6.0 新增） |
| `multiclaw config schema` | 导出配置 JSON Schema |

## 项目结构

```
multiclaw/
├── src/                    # 主库和二进制源码
│   ├── main.rs             # CLI 入口点（仅命令处理）
│   ├── lib.rs              # 库导出（所有模块声明）
│   ├── a2a/                # A2A 通信协议（v6.0 新增）
│   │   ├── mod.rs          # 模块导出
│   │   ├── protocol.rs     # 消息协议定义
│   │   └── gateway.rs      # 通信网关实现
│   ├── core/               # 核心层（v6.0 新增）
│   │   ├── mod.rs          # 模块导出
│   │   ├── memory_core.rs  # 分级记忆系统
│   │   ├── resource_core.rs# 资源管理系统
│   │   └── health_core.rs  # 健康检查系统
│   ├── agent/              # AI 代理核心逻辑
│   │   ├── chairman.rs     # 董事长 Agent（v6.0 新增）
│   │   └── ...
│   ├── skills/             # Skills 编排系统（v6.0 增强）
│   │   ├── mod.rs          # 模块导出
│   │   ├── orchestration.rs# 编排引擎
│   │   ├── skill_types.rs  # 技能类型定义
│   │   └── compat.rs       # 兼容层
│   ├── channels/           # 通信渠道实现
│   ├── providers/          # AI 提供商实现
│   ├── memory/             # 内存/存储后端
│   ├── tools/              # 工具定义和实现
│   ├── config/             # 配置解析和管理
│   ├── security/           # 安全策略和沙箱
│   ├── gateway/            # HTTP/WebSocket 网关
│   ├── daemon/             # 守护进程逻辑
│   ├── auth/               # 认证和 OAuth
│   ├── hardware/           # 硬件发现
│   ├── peripherals/        # 外设管理
│   └── ...                 # 其他模块
├── crates/
│   └── robot-kit/          # 机器人硬件工具包子 crate
├── tests/                  # 集成测试
├── benches/                # 基准测试
├── docs/                   # 文档
├── scripts/                # 开发和 CI 脚本
├── Cargo.toml              # 工作区配置
└── deny.toml               # cargo-deny 安全审计配置
```

## 开发约定

### 代码风格
- **Clippy**: 使用 `clippy::all` 和 `clippy::pedantic` 级别
- **Unsafe 代码**: `#![forbid(unsafe_code)]` 禁止使用
- **格式化**: 使用标准 rustfmt
- **Cognitive complexity 阈值**: 30
- **函数参数数量阈值**: 10
- **函数行数阈值**: 200

### 模块组织（重要）

#### 单一声明原则
**所有模块只在 `lib.rs` 中声明一次**，`main.rs` 通过 `use` 导入：

```rust
// lib.rs - 模块声明
pub mod a2a;
pub mod agent;
pub mod core;
pub mod skills;
// ... 其他模块

// main.rs - 从 lib 导入
use multiclaw::{
    a2a, agent, auth, channels, config, coordination, core,
    // ... 其他需要的模块
};
```

#### 禁止的做法
```rust
// ❌ 错误：不要在 main.rs 中声明模块
mod agent;  // 这会导致命名空间隔离

// ❌ 错误：不要使用重导出块
mod a2a {
    pub use multiclaw::a2a::*;  // 这是 workaround，不是正确做法
}

// ❌ 错误：不要重复定义类型
enum MemoryCommands { ... }  // 如果 lib.rs 已定义，main.rs 不应再定义
```

#### 正确的做法
```rust
// ✅ 正确：从 lib 导入模块
use multiclaw::{a2a, agent, core};

// ✅ 正确：重导出 lib 中的类型
pub use multiclaw::MemoryCommands;

// ✅ 正确：类型定义只在 lib.rs 中
// lib.rs
pub enum MemoryCommands { ... }

// main.rs
use multiclaw::MemoryCommands;
```

### 模块可见性规则
- `pub mod` - 公共模块，可供外部 crate 和 main.rs 使用
- `pub(crate) mod` - 仅限当前 crate 内部使用
- 如果 `main.rs` 需要使用某个模块，该模块必须是 `pub`

### 类型定义规则
- **命令枚举**：定义在 `lib.rs` 中（如 `ServiceCommands`, `MemoryCommands`）
- **公共类型**：定义在各自模块中，通过 `lib.rs` 重导出
- **避免重复**：同一类型只定义一次，通过 `use` 导入

### 错误处理
- 使用 `anyhow::Result` 作为通用错误类型
- 使用 `thiserror` 定义自定义错误类型

### 异步代码
- 使用 Tokio 运行时
- `#[tokio::main]` 在 `main.rs` 中
- 异步 trait 使用 `async-trait` crate

### 配置管理
- 配置文件: `~/.multiclaw/config.toml`
- 环境变量覆盖支持 (如 `MULTICLAW_PROVIDER`, `MULTICLAW_CONFIG_DIR`)
- JSON Schema 可通过 `multiclaw config schema` 导出

### 工作区配置
项目使用 Cargo 工作区，配置在 `Cargo.toml` 中：
```toml
[workspace]
members = ["crates/robot-kit"]
resolver = "2"
```

**注意**：不要在 `members` 中包含 `"."`，这会导致模块路径解析问题。

## Feature Flags

| Feature | 描述 |
|---------|------|
| `default` | 默认启用 `wasm-tools` |
| `wasm-tools` | WASM 插件引擎（WASI stdio 协议） |
| `hardware` | USB 硬件发现 |
| `channel-matrix` | Matrix 客户端支持（E2EE） |
| `channel-lark` | Lark/飞书渠道 |
| `memory-postgres` | PostgreSQL 内存后端 |
| `observability-otel` | OpenTelemetry 集成 |
| `browser-native` | Fantoccini 浏览器自动化 |
| `whatsapp-web` | WhatsApp Web 客户端 |
| `probe` | probe-rs STM32/Nucleo 内存读取 |
| `rag-pdf` | PDF 提取用于 RAG |
| `peripheral-rpi` | Raspberry Pi GPIO |
| `sandbox-landlock` | Landlock 沙箱（Linux） |

## 关键文档

- `README.md` - 项目概述和快速开始
- `MULTICLAW_V6_ARCHITECTURE_SUMMARY.md` - v6.0 架构总结
- `docs/commands-reference.md` - CLI 命令参考
- `docs/config-reference.md` - 配置参考
- `docs/architecture.svg` - 架构图
- `docs/troubleshooting.md` - 故障排除
- `docs/security/` - 安全文档
- `docs/hardware/` - 硬件文档
- `docs/contributing/` - 贡献指南

## 常见问题排查

### 模块找不到错误
```
error[E0432]: unresolved import `crate::a2a`
```
**原因**：`main.rs` 声明了独立模块，导致命名空间隔离。
**解决**：移除 `main.rs` 中的 `mod` 声明，改为 `use multiclaw::...` 导入。

### 类型不匹配错误
```
error[E0308]: expected `multiclaw::MemoryCommands`, found `MemoryCommands`
```
**原因**：类型在 `main.rs` 和 `lib.rs` 中重复定义。
**解决**：删除重复定义，使用 `pub use multiclaw::MemoryCommands;` 导入。

### 私有模块访问错误
```
error[E0603]: module `auth` is private
```
**原因**：模块是 `pub(crate)`，但 `main.rs` 需要使用。
**解决**：在 `lib.rs` 中将模块改为 `pub`。

## 注意事项

1. **编译资源需求**: 发布构建需要至少 2GB RAM + swap，推荐 4GB+
2. **低内存设备**: 使用 `--profile release-fast` 或 `codegen-units = 8` 加速编译
3. **发布二进制大小**: ~8.8MB（strip + LTO + opt-level=z）
4. **配置安全**: 环境变量优先级高于配置文件
5. **渠道要求**: Telegram、Discord 等渠道需要 daemon 运行
6. **模块声明**: 所有模块在 `lib.rs` 中声明，`main.rs` 只负责 CLI 处理