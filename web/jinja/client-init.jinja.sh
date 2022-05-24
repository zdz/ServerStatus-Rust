#!/bin/bash
# ServerStatus-Rust client init script

export SSR_PASS={{pass}}
export SSR_UID={{uid}}
export SSR_GID={{gid}}
export SSR_ALIAS={{alias}}
export SSR_SCHEME={{scheme}}
export SSR_DOMAIN={{domain}}
export SSR_SRVEL_URL={{server_url}}
export SSR_VNSTAT={{vnstat}}
export SSR_WEIGHT={{weight}}
export SSR_PKG_VERSION={{pkg_version}}
export SSR_CLIENT_OPTS='{{client_opts}}'
export SSR_WORKSPACE={{workspace}}

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

function download_client() {
    need_cmd rm
    need_cmd unzip
    need_cmd wget
    need_cmd chmod

    if [ "${CN}" = true ]; then
        MIRROR="https://gh-proxy.com/"
        say "using mirror: ${MIRROR}"
    fi

    cd ${SSR_WORKSPACE}
    rm -rf client-*.zip stat_* | true

    say "start downloading the stat_client"
    wget --no-check-certificate -qO "client-${arch}-unknown-linux-musl.zip" "${MIRROR}https://github.com/zdz/ServerStatus-Rust/releases/download/v{{pkg_version}}/client-${arch}-unknown-linux-musl.zip"

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
download_client
install_client_service
