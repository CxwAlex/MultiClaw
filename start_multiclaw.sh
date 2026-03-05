#!/bin/bash

# MultiClaw 启动脚本
# 用于启动和管理 MultiClaw 智能体实例

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "🚀 启动 MultiClaw 智能体系统..."
echo "📁 项目路径: $PROJECT_ROOT"
echo ""

# 检查是否已配置
if [ ! -f "$HOME/.multiclaw/config.toml" ]; then
    echo "⚠️  配置文件不存在，正在初始化..."
    cargo run --bin multiclaw -- onboard --interactive
fi

echo "✅ 配置检查完成"
echo ""

echo "📋 当前状态:"
cargo run --bin multiclaw -- status
echo ""

echo "💡 使用示例:"
echo "   # 与智能体对话"
echo "   cargo run --bin multiclaw -- agent -m \"你好\""
echo ""
echo "   # 启动守护进程"
echo "   cargo run --bin multiclaw -- daemon"
echo ""
echo "   # 查看更多命令"
echo "   cargo run --bin multiclaw -- --help"
echo ""

echo "🎯 系统已就绪！当前配置使用阿里云 DashScope 的 qwen-max 模型。"
echo "🔒 安全沙盒、自动上下文管理和协同进化功能已启用。"