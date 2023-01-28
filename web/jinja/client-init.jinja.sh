#!/bin/bash
# ServerStatus-Rust client init script

export SSR_PASS={{pass}}
export SSR_UID={{uid}}
export SSR_GID={{gid}}
export SSR_ALIAS={{alias}}
export SSR_SCHEME={{scheme}}
export SSR_DOMAIN={{domain}}
export SSR_SERVER_URL={{server_url}}
export SSR_VNSTAT={{vnstat}}
export SSR_WEIGHT={{weight}}
export SSR_PKG_VERSION={{pkg_version}}
export SSR_CLIENT_OPTS='{{client_opts}}'
export SSR_WORKSPACE={{workspace}}
export SSR_CN={{cn}}

Info="\033[32m[info]\033[0m"
Error="\033[31m[err]\033[0m"

mkdir -p ${SSR_WORKSPACE}
cd ${SSR_WORKSPACE}

if [ "${DBG}" = "1" ]; then
    set -x
fi

function say() {
    printf "${Info} ssr-client-init: %s\n" "$1"
}

function err() {
    printf "${Error} ssr-client-init: %s\n" "$1" >&2
    exit 1
}

function check_cmd() {
    command -v "$1" > /dev/null 2>&1
}

function need_cmd() {
    if ! check_cmd "$1"; then
        err "need '$1' (command not found)"
    fi
}

# check arch
function check_arch() {
    need_cmd uname

    case $(uname -m) in
        x86_64)
            arch=x86_64
        ;;
        aarch64 | aarch64_be | arm64 | armv8b | armv8l)
            arch=aarch64
        ;;
        *)
            err "暂不支持该系统架构"
            exit 1
        ;;
    esac

    say "os arch: ${arch}"
}

function install_deps() {
    say "checking dependencies"

    if [ "${SSR_VNSTAT}" == "true" ]; then
        cmd_deps=("unzip" "wget" "chmod" "vnstat")
    else
        cmd_deps=("unzip" "wget" "chmod")
    fi
    need_deps=""
    for i in "${!cmd_deps[@]}"; do
        cur_dep="${cmd_deps[$i]}"
        if [ ! -x "$(command -v $cur_dep 2>/dev/null)" ]; then
            say "$cur_dep 未安装"
            need_deps="$cur_dep ${need_deps}"
        fi
    done
    if [ "${need_deps}" ]; then
        say "start installing dependencies: ${need_deps}"

        if [ -x "$(command -v apk 2>/dev/null)" ]; then
            apk update > /dev/null 2>&1
            apk --no-cache add procps iproute2 coreutils ${need_deps} > /dev/null 2>&1
        elif [ -x "$(command -v apt-get 2>/dev/null)" ]; then
            apt-get update -y > /dev/null 2>&1
            apt-get install -y  ${need_deps} > /dev/null 2>&1
        elif [ -x "$(command -v yum 2>/dev/null)" ]; then
            yum install -y  ${need_deps} > /dev/null 2>&1
        else
            err "未找到合适的包管理工具,请手动安装: ${need_deps}"
            exit 1
        fi
        for i in "${!cmd_deps[@]}"; do
            cur_dep="${cmd_deps[$i]}"
            if [ ! -x "$(command -v $cur_dep)" ]; then
                err "$cur_dep 未成功安装,请尝试手动安装!"
                exit 1
            fi
        done
    fi
}

function download_client() {

    cd ${SSR_WORKSPACE}
    rm -rf client-*.zip stat_* | true

    say "start download the stat_client"

    if [ "${SSR_CN}" = true ]; then
        say "using cn mirror: coding.net"
        wget --no-check-certificate -qO "client-${arch}-unknown-linux-musl.zip" "https://d0ge-generic.pkg.coding.net/ServerStatus-Rust/releases/client-${arch}-unknown-linux-musl-v{{pkg_version}}.zip?version=v{{pkg_version}}"
    else
        wget --no-check-certificate -qO "client-${arch}-unknown-linux-musl.zip" "https://github.com/zdz/ServerStatus-Rust/releases/download/v{{pkg_version}}/client-${arch}-unknown-linux-musl.zip"
    fi

    say "download stat_client succ"

    say "try stop stat_client.service"
    systemctl stop stat_client > /dev/null | true

    say "unzip client-${arch}-unknown-linux-musl.zip"
    unzip -o client-${arch}-unknown-linux-musl.zip
    rm -rf stat_client.service | true

    chmod +x ${SSR_WORKSPACE}/stat_client
}

function install_client_service() {
    need_cmd cat
    need_cmd systemctl
    need_cmd sleep

    say "start install stat_client.service"

    cat > /etc/systemd/system/stat_client.service <<-EOF
[Unit]
Description=ServerStatus-Rust Client
After=network.target

[Service]
User=root
Group=root
Environment="RUST_BACKTRACE=1"
WorkingDirectory={{workspace}}
ExecStart={{workspace}}/stat_client {{client_opts}}
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure

[Install]
WantedBy=multi-user.target

EOF

    say "systemctl daemon-reload"
    systemctl daemon-reload
    say "start stat_client.service"
    systemctl start stat_client
    say "enable stat_client.service"
    systemctl enable stat_client

    sleep 2
    say "status stat_client.service"
    systemctl status stat_client

}

check_arch
install_deps
download_client
install_client_service
