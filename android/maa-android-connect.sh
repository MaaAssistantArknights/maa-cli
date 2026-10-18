#!/usr/bin/env bash
set -uo pipefail

usage() {
    cat <<'EOF'
maa-android-connect —— 在 Termux 里把 MAA 自动连到本机 adb

按成功率从高到低依次尝试 mDNS 发现、root 设固定端口、扫描常见端口。
连接原理与手动步骤见：
https://docs.maa.plus/zh-cn/manual/cli/install.html#android-termux

用法：
  ./maa-android-connect.sh            # 自动尝试所有方式
  ./maa-android-connect.sh --root     # 只试 root 方式（设固定端口）
  ./maa-android-connect.sh --mdns     # 只试 mDNS 发现（已配对过的设备）
  ./maa-android-connect.sh --verbose  # 打印更多过程

退出码：0 = 已连上；1 = 全部失败
EOF
}

# 部分设备/模拟器会在这些端口上监听 adb
COMMON_PORTS="5555 5556 5557 7555 16384 16385"

ONLY=""
VERBOSE=0
for arg in "$@"; do
    case "$arg" in
        --root) ONLY="root" ;;
        --mdns) ONLY="mdns" ;;
        --verbose) VERBOSE=1 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "未知参数：$arg（用 --help 查看用法）" >&2; exit 2 ;;
    esac
done

C_RESET=$'\033[0m'; C_CYAN=$'\033[36m'; C_GREEN=$'\033[32m'
C_YELLOW=$'\033[33m'; C_RED=$'\033[31m'
log()  { printf '%s[*]%s %s\n' "$C_CYAN" "$C_RESET" "$*"; }
ok()   { printf '%s[+]%s %s\n' "$C_GREEN" "$C_RESET" "$*"; }
warn() { printf '%s[!]%s %s\n' "$C_YELLOW" "$C_RESET" "$*"; }
err()  { printf '%s[x]%s %s\n' "$C_RED" "$C_RESET" "$*"; }
trace() { [ "$VERBOSE" -eq 1 ] && printf '    %s\n' "$*"; return 0; }

if ! command -v adb >/dev/null 2>&1; then
    err "未找到 adb。请先安装：pkg install -y android-tools"
    exit 1
fi

connected() {
    adb devices 2>/dev/null | awk 'NR>1 && $2=="device" { n++ } END { exit !(n>0) }'
}
show_devices() {
    adb devices 2>/dev/null | sed 's/^/    /'
}

if connected; then
    ok "已有可用的 adb 设备，无需重连"
    show_devices
    exit 0
fi
log "当前没有可用设备，开始尝试自动连接"

try_mdns() {
    log "方式 1：mDNS 发现无线调试服务"
    command -v adb >/dev/null || return 1

    # 启动本地 adb server（mdns 发现需要它）
    adb start-server >/dev/null 2>&1

    local found
    found="$(adb mdns services 2>/dev/null \
             | awk '$2 ~ /_adb-tls-connect/ { print $3 }' | head -5)"
    trace "发现的服务：${found:-（无）}"

    if [ -z "$found" ]; then
        warn "未发现无线调试服务（可能未开启无线调试，或此前未配对过）"
        return 1
    fi

    local addr
    while read -r addr; do
        [ -n "$addr" ] || continue
        log "尝试连接 $addr"
        adb connect "$addr" >/dev/null 2>&1
        sleep 1
        if connected; then
            ok "已通过 mDNS 连接到 $addr"
            show_devices
            return 0
        fi
    done <<< "$found"

    warn "mDNS 发现的地址均未能连接（多半是尚未配对，需先 adb pair）"
    return 1
}

have_root() {
    command -v su >/dev/null 2>&1 || return 1
    su -c id 2>/dev/null | grep -q 'uid=0'
}

try_root() {
    log "方式 2：使用 root 设固定 adb 端口"

    if ! have_root; then
        warn "没有可用的 su（root），跳过"
        return 1
    fi

    # 注意：重启 adbd 会短暂断开现有 adb 连接，故放在此处（前面已确认无设备）
    log "设置 service.adb.tcp.port=5555 并重启 adbd"
    su -c 'setprop service.adb.tcp.port 5555' 2>/dev/null
    if ! su -c 'setprop ctl.restart adbd' 2>/dev/null; then
        # 部分设备不支持 ctl.restart，退回 stop/start
        trace "ctl.restart 不可用，改用 stop/start"
        su -c 'stop adbd' 2>/dev/null
        sleep 1
        su -c 'start adbd' 2>/dev/null
    fi

    for _ in 1 2 3 4 5 6 7 8 9 10; do
        sleep 1
        adb connect 127.0.0.1:5555 >/dev/null 2>&1
        if connected; then
            ok "已通过 root 连接到 127.0.0.1:5555"
            show_devices
            return 0
        fi
    done

    warn "root 方式未成功（设备可能限制 setprop adb 端口）"
    return 1
}

try_ports() {
    log "方式 3：扫描常见 adb 端口"
    local port
    for port in $COMMON_PORTS; do
        trace "尝试 127.0.0.1:$port"
        adb connect "127.0.0.1:$port" >/dev/null 2>&1
        sleep 1
        if connected; then
            ok "已连接到 127.0.0.1:$port"
            show_devices
            return 0
        fi
    done
    warn "常见端口均不可用"
    return 1
}

case "$ONLY" in
    root) try_root ;;
    mdns) try_mdns ;;
    *)    try_mdns || try_root || try_ports ;;
esac

if connected; then
    exit 0
fi

echo
err "自动连接失败，手动连接步骤见："
err "  https://docs.maa.plus/zh-cn/manual/cli/install.html#android-termux"
exit 1
