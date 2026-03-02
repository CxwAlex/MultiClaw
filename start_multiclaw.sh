#!/bin/bash

# multiclaw 启动脚本
# 用于启动 multiclaw 实例（演示模式）

set -e

echo "🚀 启动 MultiClaw 实例..."

# 编译项目
echo "📦 编译项目..."
cd /Users/god/Documents/agent/multiclaw-workspace/multiclaw-target
cargo build --release

echo ""
echo "✅ MultiClaw 实例已成功构建！"
echo ""
echo "重要提示：系统已实现以下核心功能："
echo "1. 自动上下文管理 - 防止长时间运行时的上下文溢出"
echo "2. 协同进化能力 - Agent 间的经验共享和策略提炼" 
echo "3. WASM 技能沙盒 - 安全的第三方技能执行环境"
echo "4. 五层权限架构 - User→Chairman→CEO→Team→Agent"
echo ""
echo "要完整体验所有功能，您需要："
echo "1. 获取 OpenRouter 或其他支持的提供商的 API 密钥"
echo "2. 运行：cargo run --bin multiclaw -- onboard --api-key YOUR_API_KEY --provider PROVIDER_NAME"
echo "3. 选择支持的模型，例如：anthropic/claude-sonnet-4.6"
echo ""
echo "当前系统状态："
cargo run --bin multiclaw -- status

echo ""
echo "💡 您可以使用以下命令开始配置："
echo "cargo run --bin multiclaw -- onboard --interactive"
echo ""
echo "系统已准备好，等待您的 API 密钥进行完整配置。"