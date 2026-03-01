#!/bin/bash
# ============================================================
# MultiClaw 多实例管理脚本
# ============================================================
# 版本：1.1
# 更新日期：2026-02-28
# 验证状态：✅ 已验证（multiclaw 原生支持 --config-dir）
# 文档参考：MULTI_INSTANCE_SETUP.md
#
# 技术说明：
# - multiclaw 所有命令支持 --config-dir 参数
# - 每个实例运行在独立进程，完全隔离
# - 配置、记忆、工作空间完全独立
# ============================================================

set -e

# ==================== 配置 ====================
MULTICLAW_BIN="${MULTICLAW_BIN:-./target/release/multiclaw}"
INSTANCES_DIR="${INSTANCES_DIR:-$HOME/.multiclaw_instances}"
DEFAULT_API_KEY="${DASHSCOPE_API_KEY:-}"

# 默认配置值
DEFAULT_PROVIDER="qwen-coding-plan"
DEFAULT_MODEL="qwen3.5-plus"
FALLBACK_PROVIDER="qwen-code"
FALLBACK_MODEL="qwen3-coder-plus"

# 子 Agent 配置
ARCHITECT_PROVIDER="qwen-coding-plan"
ARCHITECT_MODEL="glm-5"
RESEARCHER_PROVIDER="qwen-coding-plan"
RESEARCHER_MODEL="kimi-k2.5"
SENIOR_DEV_PROVIDER="qwen-coding-plan"
SENIOR_DEV_MODEL="qwen3.5-plus"
JUNIOR_DEV_PROVIDER="qwen-code"
JUNIOR_DEV_MODEL="qwen3-coder-plus"

# 限额配置
MAX_ACTIONS_PER_HOUR=-1
MAX_COST_PER_DAY_CENTS=240000

# ==================== 颜色 ====================
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

show_help() {
    cat << EOF
${GREEN}MultiClaw 多实例管理脚本 v1.1${NC}
${YELLOW}验证状态：${NC}multiclaw 原生支持 --config-dir

用法：\$0 <命令> [参数]

命令:
  create <instance_name>     创建新实例
  start <instance_name>      启动实例（使用 --config-dir）
  stop <instance_name>       停止实例
  restart <instance_name>    重启实例
  status [instance_name]     查看状态
  list                       列出所有实例
  remove <instance_name>     删除实例
  config <instance_name>     编辑实例配置
  logs <instance_name>       查看日志

示例:
  \$0 create my_project      创建名为 my_project 的实例
  \$0 start my_project       启动 my_project 实例
  \$0 list                   列出所有实例
  \$0 status                 查看所有实例状态

${YELLOW}技术说明:${NC}
- 每个实例运行在独立进程，使用 --config-dir 参数
- 实例目录：~/.multiclaw_instances/<instance_name>/
- 每个实例有独立的配置、记忆、工作空间
EOF
}

check_bin() {
    if [ ! -f "$MULTICLAW_BIN" ]; then
        log_error "MultiClaw 二进制文件不存在：$MULTICLAW_BIN"
        log_info "请先编译：cargo build --release"
        exit 1
    fi
}

create_instance() {
    local instance_name="$1"
    [ -z "$instance_name" ] && { log_error "请提供实例名称"; exit 1; }
    check_bin
    
    local instance_dir="$INSTANCES_DIR/$instance_name"
    [ -d "$instance_dir" ] && { log_error "实例已存在：$instance_name"; exit 1; }
    
    log_info "创建实例：$instance_name"
    mkdir -p "$instance_dir"/{workspace/{sessions,memory,state,cron,skills},.qwen}
    
    echo ""
    log_info "请提供 Telegram Bot Token"
    read -p "Bot Token: " bot_token
    [ -z "$bot_token" ] && { log_error "Bot Token 不能为空"; exit 1; }
    
    log_info "验证 Bot Token..."
    local bot_info=$(curl -s "https://api.telegram.org/bot${bot_token}/getMe")
    if echo "$bot_info" | grep -q '"ok":true'; then
        local bot_username=$(echo "$bot_info" | grep -o '"username":"[^"]*"' | cut -d'"' -f4)
        log_success "Bot 验证成功：@$bot_username"
    else
        log_error "Bot Token 无效"; exit 1
    fi
    
    log_info "请在 Telegram 中给 @$bot_username 发送 /start，然后按回车..."
    read
    local updates=$(curl -s "https://api.telegram.org/bot${bot_token}/getUpdates")
    local user_id=$(echo "$updates" | grep -o '"id":[0-9]*' | head -1 | cut -d':' -f2)
    local username=$(echo "$updates" | grep -o '"username":"[^"]*"' | head -1 | cut -d'"' -f4)
    [ -n "$user_id" ] && log_success "用户信息：ID=$user_id, Username=@$username" || username="your_username"
    
    log_info "生成配置文件..."
    cat > "$instance_dir/config.toml" << EOF
default_provider = "$DEFAULT_PROVIDER"
default_model = "$DEFAULT_MODEL"
default_temperature = 0.7

[observability]
backend = "none"

[autonomy]
level = "supervised"
workspace_only = true
max_actions_per_hour = $MAX_ACTIONS_PER_HOUR
max_cost_per_day_cents = $MAX_COST_PER_DAY_CENTS

[security]
otp_enabled = false

[runtime]
kind = "native"

[agent]
compact_context = true
max_tool_iterations = 50

[memory]
backend = "sqlite"
auto_save = true

[reliability]
provider_retries = 3
fallback_providers = ["$FALLBACK_PROVIDER:$FALLBACK_MODEL"]

[channels_config]
cli = true
message_timeout_secs = 600

[channels_config.telegram]
bot_token = "$bot_token"
allowed_users = ["$username", "$user_id", "*"]
mention_only = false

[agents.architect]
provider = "$ARCHITECT_PROVIDER"
model = "$ARCHITECT_MODEL"
temperature = 0.3
system_prompt = "你是资深架构设计师，擅长系统设计和评审。"
agentic = true
allowed_tools = ["file_read", "file_write", "memory_recall"]
max_iterations = 15

[agents.researcher]
provider = "$RESEARCHER_PROVIDER"
model = "$RESEARCHER_MODEL"
temperature = 0.5
system_prompt = "你是专业研究员，擅长信息搜集和整理。"
agentic = true
allowed_tools = ["web_search", "web_fetch", "memory_recall"]
max_iterations = 10

[agents.senior_dev]
provider = "$SENIOR_DEV_PROVIDER"
model = "$SENIOR_DEV_MODEL"
temperature = 0.2
system_prompt = "你是高级工程师，编写高质量代码。"
agentic = true
allowed_tools = ["file_read", "file_write", "shell", "git"]
max_iterations = 20

[agents.junior_dev]
provider = "$JUNIOR_DEV_PROVIDER"
model = "$JUNIOR_DEV_MODEL"
temperature = 0.1
system_prompt = "你是初级工程师，执行简单任务。"
agentic = true
allowed_tools = ["file_read", "file_write"]
max_iterations = 10

[cost]
enabled = true
daily_limit_usd = 100.0

[coordination]
enabled = true
EOF
    
    log_success "实例创建成功！"
    echo -e "\n${GREEN}实例信息:${NC}\n  名称：$instance_name\n  路径：$instance_dir\n  Bot: @$bot_username"
    echo -e "\n${YELLOW}下一步:${NC}\n  启动：$0 start $instance_name"
}

start_instance() {
    local instance_name="$1"
    [ -z "$instance_name" ] && { log_error "请提供实例名称"; exit 1; }
    local instance_dir="$INSTANCES_DIR/$instance_name"
    [ ! -d "$instance_dir" ] && { log_error "实例不存在"; exit 1; }
    check_bin
    [ -z "$DEFAULT_API_KEY" ] && { log_error "请设置 DASHSCOPE_API_KEY"; exit 1; }
    pgrep -f "multiclaw.*$instance_name" > /dev/null && { log_warn "已在运行"; return 0; }
    
    log_info "启动实例：$instance_name"
    log_info "配置目录：$instance_dir"
    cd "$instance_dir"
    # 使用 --config-dir 参数启动独立实例
    nohup env DASHSCOPE_API_KEY="$DEFAULT_API_KEY" "$MULTICLAW_BIN" daemon --config-dir "$instance_dir" > "$instance_dir/logs.log" 2>&1 &
    sleep 2
    pgrep -f "multiclaw.*$instance_name" > /dev/null && log_success "已启动 (PID: $(pgrep -f "multiclaw.*$instance_name"))" || log_error "启动失败"
}

stop_instance() {
    local instance_name="$1"
    [ -z "$instance_name" ] && { log_error "请提供实例名称"; exit 1; }
    local pids=$(pgrep -f "multiclaw.*$instance_name")
    [ -z "$pids" ] && { log_warn "未运行"; return 0; }
    log_info "停止实例：$instance_name"
    kill $pids 2>/dev/null; sleep 2
    pkill -9 -f "multiclaw.*$instance_name" 2>/dev/null; log_success "已停止"
}

restart_instance() { stop_instance "$1"; sleep 2; start_instance "$1"; }

show_status() {
    local instance_name="$1"
    if [ -n "$instance_name" ]; then
        local instance_dir="$INSTANCES_DIR/$instance_name"
        [ ! -d "$instance_dir" ] && { log_error "实例不存在"; exit 1; }
        echo -e "\n${GREEN}实例：$instance_name${NC}"; echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        pgrep -f "multiclaw.*$instance_name" > /dev/null && echo -e "状态：${GREEN}运行中${NC}" || echo -e "状态：${RED}已停止${NC}"
    else
        echo -e "\n${GREEN}所有实例${NC}"; echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        [ ! -d "$INSTANCES_DIR" ] && { echo "暂无实例"; return 0; }
        for dir in "$INSTANCES_DIR"/*/; do
            [ -d "$dir" ] && { local name=$(basename "$dir"); pgrep -f "multiclaw.*$name" > /dev/null && echo -e "  ${GREEN}●${NC} $name" || echo -e "  ${RED}○${NC} $name"; }
        done
    fi
}

list_instances() {
    echo -e "\n${GREEN}已注册实例${NC}"; echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    [ ! -d "$INSTANCES_DIR" ] && { echo "暂无实例"; return 0; }
    local count=0
    for dir in "$INSTANCES_DIR"/*/; do
        [ -d "$dir" ] && { echo "  $(basename "$dir")"; ((count++)); }
    done
    echo -e "\n总计：$count 个实例\n"
}

remove_instance() {
    local instance_name="$1"
    [ -z "$instance_name" ] && exit 1
    local instance_dir="$INSTANCES_DIR/$instance_name"
    [ ! -d "$instance_dir" ] && { log_error "实例不存在"; exit 1; }
    stop_instance "$instance_name"
    read -p "确认删除？(y/N): " c; [ "$c" = "y" ] || [ "$c" = "Y" ] && rm -rf "$instance_dir" && log_success "已删除" || log_info "已取消"
}

edit_config() {
    local instance_name="$1"
    [ -z "$instance_name" ] && exit 1
    local config_file="$INSTANCES_DIR/$instance_name/config.toml"
    [ ! -f "$config_file" ] && { log_error "配置不存在"; exit 1; }
    log_info "编辑配置：$config_file"
    nano "$config_file" 2>/dev/null || vi "$config_file"
    log_info "重启生效：$0 restart $instance_name"
}

show_logs() {
    local instance_name="$1"
    local lines="${2:-50}"
    [ -z "$instance_name" ] && exit 1
    local log_file="$INSTANCES_DIR/$instance_name/logs.log"
    [ ! -f "$log_file" ] && { log_warn "日志不存在"; return 0; }
    tail -n "$lines" "$log_file"
}

case "$1" in
    create) create_instance "$2" ;;
    start) start_instance "$2" ;;
    stop) stop_instance "$2" ;;
    restart) restart_instance "$2" ;;
    status) show_status "$2" ;;
    list) list_instances ;;
    remove) remove_instance "$2" ;;
    config) edit_config "$2" ;;
    logs) show_logs "$2" "$3" ;;
    *) show_help ;;
esac
