#!/bin/bash
# multiclaw-instance.sh - MultiClaw 多实例管理脚本
# 
# 用法:
#   ./multiclaw-instance.sh create <name>     # 创建新实例
#   ./multiclaw-instance.sh start <name>      # 启动实例
#   ./multiclaw-instance.sh stop <name>       # 停止实例
#   ./multiclaw-instance.sh restart <name>    # 重启实例
#   ./multiclaw-instance.sh status [name]     # 查看状态
#   ./multiclaw-instance.sh list              # 列出所有实例
#   ./multiclaw-instance.sh remove <name>     # 删除实例
#   ./multiclaw-instance.sh logs <name>       # 查看日志
#   ./multiclaw-instance.sh config <name>     # 编辑配置

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 目录配置
INSTANCES_DIR="$HOME/.multiclaw_instances"
DEFAULT_INSTANCE="$HOME/.multiclaw"
MULTICLAW_BIN="multiclaw"

# 如果是开发模式，使用 cargo run
if [[ -f "./Cargo.toml" ]] && [[ -d "./src" ]]; then
    MULTICLAW_BIN="cargo run --release --"
fi

# 打印帮助信息
print_help() {
    echo -e "${CYAN}MultiClaw 多实例管理工具${NC}"
    echo ""
    echo "用法: $0 <命令> [参数]"
    echo ""
    echo "命令:"
    echo "  create <name>      创建新实例"
    echo "  start <name>       启动实例"
    echo "  stop <name>        停止实例"
    echo "  restart <name>     重启实例"
    echo "  status [name]      查看状态（不指定名称则显示所有）"
    echo "  list               列出所有实例"
    echo "  remove <name>      删除实例"
    echo "  logs <name>        查看日志"
    echo "  config <name>      编辑配置"
    echo ""
    echo "示例:"
    echo "  $0 create project_a              # 创建名为 project_a 的实例"
    echo "  $0 start project_a               # 启动 project_a 实例"
    echo "  $0 list                          # 列出所有实例"
    echo ""
    echo "目录结构:"
    echo "  默认实例: ~/.multiclaw/"
    echo "  其他实例: ~/.multiclaw_instances/<name>/"
}

# 获取实例目录
get_instance_dir() {
    local name="$1"
    
    if [[ "$name" == "default" ]] || [[ -z "$name" ]]; then
        echo "$DEFAULT_INSTANCE"
        return
    fi
    
    echo "$INSTANCES_DIR/$name"
}

# 检查实例是否存在
instance_exists() {
    local name="$1"
    local dir=$(get_instance_dir "$name")
    [[ -f "$dir/config.toml" ]]
}

# 创建新实例
create_instance() {
    local name="$1"
    
    if [[ -z "$name" ]]; then
        echo -e "${RED}错误: 请指定实例名称${NC}"
        exit 1
    fi
    
    local instance_dir=$(get_instance_dir "$name")
    
    if [[ -d "$instance_dir" ]]; then
        echo -e "${RED}错误: 实例 '$name' 已存在${NC}"
        echo -e "  目录: $instance_dir"
        exit 1
    fi
    
    echo -e "${CYAN}创建实例 '$name'...${NC}"
    
    # 创建目录
    mkdir -p "$instance_dir"
    mkdir -p "$instance_dir/workspace"
    mkdir -p "$instance_dir/workspace/sessions"
    mkdir -p "$instance_dir/workspace/memory"
    mkdir -p "$instance_dir/workspace/state"
    mkdir -p "$instance_dir/workspace/cron"
    mkdir -p "$instance_dir/workspace/skills"
    mkdir -p "$instance_dir/workspace/instances"
    
    # 运行 onboard 初始化
    echo -e "${CYAN}初始化配置...${NC}"
    $MULTICLAW_BIN onboard --config-dir "$instance_dir"
    
    echo ""
    echo -e "${GREEN}✅ 实例 '$name' 创建成功！${NC}"
    echo -e "   目录: ${CYAN}$instance_dir${NC}"
    echo ""
    echo "启动实例:"
    echo -e "  ${YELLOW}$MULTICLAW_BIN daemon --config-dir $instance_dir${NC}"
    echo ""
    echo "或使用此脚本:"
    echo -e "  ${YELLOW}$0 start $name${NC}"
}

# 启动实例
start_instance() {
    local name="$1"
    
    if [[ -z "$name" ]]; then
        echo -e "${RED}错误: 请指定实例名称${NC}"
        exit 1
    fi
    
    if ! instance_exists "$name"; then
        echo -e "${RED}错误: 实例 '$name' 不存在${NC}"
        exit 1
    fi
    
    local instance_dir=$(get_instance_dir "$name")
    
    echo -e "${CYAN}启动实例 '$name'...${NC}"
    
    # 检查是否已经在运行
    if pgrep -f "multiclaw daemon.*--config-dir.*$instance_dir" > /dev/null 2>&1; then
        echo -e "${YELLOW}警告: 实例 '$name' 可能已在运行${NC}"
    fi
    
    # 启动 daemon
    $MULTICLAW_BIN daemon --config-dir "$instance_dir"
}

# 停止实例
stop_instance() {
    local name="$1"
    
    if [[ -z "$name" ]]; then
        echo -e "${RED}错误: 请指定实例名称${NC}"
        exit 1
    fi
    
    local instance_dir=$(get_instance_dir "$name")
    
    echo -e "${CYAN}停止实例 '$name'...${NC}"
    
    # 查找并杀死进程
    local pids=$(pgrep -f "multiclaw daemon.*--config-dir.*$instance_dir" 2>/dev/null || true)
    
    if [[ -z "$pids" ]]; then
        echo -e "${YELLOW}实例 '$name' 未在运行${NC}"
        return
    fi
    
    for pid in $pids; do
        echo -e "  停止进程 ${YELLOW}$pid${NC}"
        kill "$pid" 2>/dev/null || true
    done
    
    sleep 2
    
    # 强制杀死仍在运行的进程
    for pid in $pids; do
        if kill -0 "$pid" 2>/dev/null; then
            echo -e "  强制停止进程 ${RED}$pid${NC}"
            kill -9 "$pid" 2>/dev/null || true
        fi
    done
    
    echo -e "${GREEN}✅ 实例 '$name' 已停止${NC}"
}

# 重启实例
restart_instance() {
    local name="$1"
    
    if [[ -z "$name" ]]; then
        echo -e "${RED}错误: 请指定实例名称${NC}"
        exit 1
    fi
    
    stop_instance "$name"
    sleep 1
    start_instance "$name"
}

# 查看实例状态
status_instance() {
    local name="$1"
    local instance_dir=$(get_instance_dir "$name")
    
    if [[ ! -f "$instance_dir/config.toml" ]]; then
        echo -e "${RED}实例 '$name' 不存在${NC}"
        return
    fi
    
    echo -e "${CYAN}实例: $name${NC}"
    echo -e "  目录: $instance_dir"
    
    # 检查进程状态
    local pids=$(pgrep -f "multiclaw daemon.*--config-dir.*$instance_dir" 2>/dev/null || true)
    
    if [[ -n "$pids" ]]; then
        echo -e "  状态: ${GREEN}运行中${NC}"
        echo -e "  PID: ${YELLOW}$pids${NC}"
        
        # 尝试获取健康状态
        local health_url="http://127.0.0.1:8080/health"
        if command -v curl &> /dev/null; then
            local health=$(curl -s "$health_url" 2>/dev/null || echo "无法连接")
            echo -e "  健康: $health"
        fi
    else
        echo -e "  状态: ${RED}未运行${NC}"
    fi
    
    # 显示配置摘要
    if [[ -f "$instance_dir/config.toml" ]]; then
        local provider=$(grep -E "^default_provider" "$instance_dir/config.toml" 2>/dev/null | cut -d'=' -f2 | tr -d ' "' || echo "未设置")
        local model=$(grep -E "^default_model" "$instance_dir/config.toml" 2>/dev/null | cut -d'=' -f2 | tr -d ' "' || echo "未设置")
        echo -e "  Provider: ${CYAN}$provider${NC}"
        echo -e "  Model: ${CYAN}$model${NC}"
    fi
    
    echo ""
}

# 列出所有实例
list_instances() {
    echo -e "${CYAN}📋 MultiClaw 实例列表:${NC}"
    echo ""
    
    # 默认实例
    if [[ -f "$DEFAULT_INSTANCE/config.toml" ]]; then
        echo -e "  ${GREEN}[默认]${NC} ~/.multiclaw"
        local pids=$(pgrep -f "multiclaw daemon.*--config-dir.*$DEFAULT_INSTANCE" 2>/dev/null || true)
        if [[ -n "$pids" ]]; then
            echo -e "         状态: ${GREEN}运行中${NC} (PID: $pids)"
        else
            echo -e "         状态: ${RED}未运行${NC}"
        fi
        echo ""
    fi
    
    # 其他实例
    if [[ -d "$INSTANCES_DIR" ]]; then
        local found=0
        for dir in "$INSTANCES_DIR"/*; do
            if [[ -d "$dir" ]] && [[ -f "$dir/config.toml" ]]; then
                found=1
                local name=$(basename "$dir")
                echo -e "  ${BLUE}[$name]${NC} $dir"
                local pids=$(pgrep -f "multiclaw daemon.*--config-dir.*$dir" 2>/dev/null || true)
                if [[ -n "$pids" ]]; then
                    echo -e "         状态: ${GREEN}运行中${NC} (PID: $pids)"
                else
                    echo -e "         状态: ${RED}未运行${NC}"
                fi
                echo ""
            fi
        done
        
        if [[ $found -eq 0 ]]; then
            echo -e "  ${YELLOW}没有其他实例${NC}"
            echo ""
        fi
    else
        echo -e "  ${YELLOW}没有其他实例${NC}"
        echo ""
    fi
    
    echo "使用方法:"
    echo -e "  启动实例: ${YELLOW}$0 start <name>${NC}"
    echo -e "  创建实例: ${YELLOW}$0 create <name>${NC}"
}

# 删除实例
remove_instance() {
    local name="$1"
    
    if [[ -z "$name" ]]; then
        echo -e "${RED}错误: 请指定实例名称${NC}"
        exit 1
    fi
    
    if [[ "$name" == "default" ]]; then
        echo -e "${RED}错误: 不能删除默认实例${NC}"
        exit 1
    fi
    
    local instance_dir=$(get_instance_dir "$name")
    
    if [[ ! -d "$instance_dir" ]]; then
        echo -e "${RED}错误: 实例 '$name' 不存在${NC}"
        exit 1
    fi
    
    # 先停止实例
    stop_instance "$name" 2>/dev/null || true
    
    echo -e "${YELLOW}警告: 即将删除实例 '$name'${NC}"
    echo -e "  目录: $instance_dir"
    echo ""
    read -p "确认删除? (y/N) " -n 1 -r
    echo ""
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf "$instance_dir"
        echo -e "${GREEN}✅ 实例 '$name' 已删除${NC}"
    else
        echo -e "${YELLOW}取消删除${NC}"
    fi
}

# 查看日志
logs_instance() {
    local name="$1"
    
    if [[ -z "$name" ]]; then
        echo -e "${RED}错误: 请指定实例名称${NC}"
        exit 1
    fi
    
    local instance_dir=$(get_instance_dir "$name")
    local log_dir="$instance_dir/logs"
    
    if [[ -d "$log_dir" ]]; then
        echo -e "${CYAN}实例 '$name' 日志:${NC}"
        echo ""
        if [[ -f "$log_dir/daemon.stdout.log" ]]; then
            echo -e "${CYAN}=== STDOUT ===${NC}"
            tail -50 "$log_dir/daemon.stdout.log"
        fi
        if [[ -f "$log_dir/daemon.stderr.log" ]]; then
            echo ""
            echo -e "${RED}=== STDERR ===${NC}"
            tail -50 "$log_dir/daemon.stderr.log"
        fi
    else
        echo -e "${YELLOW}没有找到日志目录${NC}"
    fi
}

# 编辑配置
config_instance() {
    local name="$1"
    
    if [[ -z "$name" ]]; then
        echo -e "${RED}错误: 请指定实例名称${NC}"
        exit 1
    fi
    
    local instance_dir=$(get_instance_dir "$name")
    local config_file="$instance_dir/config.toml"
    
    if [[ ! -f "$config_file" ]]; then
        echo -e "${RED}错误: 实例 '$name' 配置文件不存在${NC}"
        exit 1
    fi
    
    # 使用 EDITOR 或默认编辑器
    local editor="${EDITOR:-nano}"
    $editor "$config_file"
}

# 主命令处理
case "${1:-}" in
    create)
        create_instance "$2"
        ;;
    start)
        start_instance "$2"
        ;;
    stop)
        stop_instance "$2"
        ;;
    restart)
        restart_instance "$2"
        ;;
    status)
        if [[ -n "$2" ]]; then
            status_instance "$2"
        else
            list_instances
        fi
        ;;
    list)
        list_instances
        ;;
    remove)
        remove_instance "$2"
        ;;
    logs)
        logs_instance "$2"
        ;;
    config)
        config_instance "$2"
        ;;
    help|--help|-h)
        print_help
        ;;
    *)
        if [[ -n "${1:-}" ]]; then
            echo -e "${RED}未知命令: $1${NC}"
            echo ""
        fi
        print_help
        exit 1
        ;;
esac