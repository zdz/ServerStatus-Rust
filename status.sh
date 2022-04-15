#!/usr/bin/env bash
#=================================================
#  Description: Serverstat-Rust
#  Version: v1.0.0
#=================================================

Info="\033[32m[信息]\033[0m"
Error="\033[31m[错误]\033[0m"
Tip="\033[32m[注意]\033[0m"

client_dir=/usr/local/ServerStatus/client/
server_dir=/usr/local/ServerStatus/server/
client_conf=/lib/systemd/system/stat-client.service
server_conf=/lib/systemd/system/stat-server.service

function sshelp() {  
    printf "
help:\n\
    -i,--install    安装 Status\n\
        -i -s           安装 Server\n\
        -i -c           安装 Client\n\
        -i -c conf      自动安装 Client\n\
    -u,--uninstall  卸载 Status\n\
        -u -s           卸载 Server\n\
        -u -c           卸载 Client\n\
    -r,--reset      更改 Status 配置\n\
        -r          更改 Client 配置\n\
        -r conf         自动更改 Client配置\n\
    -s,--server     管理 Status 运行状态\n\
        -s {start|stop|restart}\n\
    -c,--client     管理 Client 运行状态\n\
        -c {start|stop|restart}\n\
\n"
}

# 检查架构
function check_arch() {
    case $(uname -m) in
        x86_64)
            arch=x86_64
        ;;
        aarch64 | aarch64_be | arm64 | armv8b | armv8l)
            arch=aarch64
        ;;
        *)
            echo -e "${Error} 暂不支持该系统架构"
            exit 1
        ;;
    esac
}

check_arch

# 检查发行版
function check_release() {
    if [[ -f /etc/redhat-release ]]; then
        release="rpm"
    elif grep -q -E -i "centos|red hat|redhat" /proc/version; then
        release="rpm"
    elif grep -q -E -i "centos|red hat|redhat" /etc/issue; then
        release="rpm"
    elif grep -q -E -i "debian|ubuntu" /etc/issue; then
        release="deb"
    elif grep -q -E -i "debian|ubuntu" /proc/version; then
        release="deb"
    else
        echo -e "${Error} 暂不支持该 Linux 发行版"
        exit 1
    fi
}

check_release

function install_tool() {
  if [[ ${release} == "rpm" ]]; then
    yum -y install unzip wget
  elif [[ ${release} == "deb" ]]; then
    apt -y update
    apt -y install unzip wget
  fi
}

# 获取服务端信息
function input_upm() {
    echo -e "${Tip} 请输入服务端的信息, 格式为 \"protocol://username:password@master:port\""
    read -re UPM
}

function get_conf() {
    PROTOCOL=$(echo "${UPM}" |sed "s/\///g" |awk -F "[:@]" '{print $1}')
    USER=$(echo "${UPM}" |sed "s/\///g" |awk -F "[:@]" '{print $2}')
    PASSWD=$(echo "${UPM}" |sed "s/\///g" |awk -F "[:@]" '{print $3}')
    if [ "${PROTOCOL}" = "tcp" ]; then
        echo -e "${Info} 使用 tcp 连接"
        MASTER=$(echo "${UPM}" |awk -F "[@]" '{print $2}')
    else
        echo -e "${Info} 使用 http 连接"
        MASTER=$(echo "${UPM}" |awk -F "[@]" '{print $2}')/report
    fi
}

# 检查服务
check_server() {
    SPID=$(pgrep -f "stat-server")
}
check_client() {
    CPID=$(pgrep -f "stat-client")
}

# 写入 systemd 配置
function write_server() {
    echo -e "${Info} 写入systemd配置中"
    cat >${server_conf} <<-EOF
[Unit]
Description=ServerStatus-Rust Server
After=network.target

[Service]
#User=nobody
#Group=nobody
Environment="RUST_BACKTRACE=1"
WorkingDirectory=/usr/local/ServerStatus
ExecStart=/usr/local/ServerStatus/server/stat-server -c /usr/local/ServerStatus/server/config.toml
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure

[Install]
WantedBy=multi-user.target
EOF
}
function write_client() {
    echo -e "${Info} 写入systemd配置中"
    cat >${client_conf} <<-EOF
[Unit]
Description=Serverstat-Rust Client
After=network.target

[Service]
User=root
Group=root
Environment="RUST_BACKTRACE=1"
WorkingDirectory=/usr/local/ServerStatus
ExecStart=/usr/local/ServerStatus/client/stat-client -a "${PROTOCOL}://${MASTER}" -u ${USER} -p ${PASSWD}
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure

[Install]
WantedBy=multi-user.target
EOF
}

# systemd 操作
function ssserver() {
    INCMD="$1"; shift
    case ${INCMD} in
        stop)
            systemctl stop stat-server
        ;;
        start)
            systemctl start stat-server
        ;;
        restart)
            systemctl restart stat-server
        ;;
        *)
            sshelp
        ;;
  esac
}
function ssclient() {
    INCMD="$1"; shift
    case ${INCMD} in
        stop)
            systemctl stop stat-client
        ;;
        start)
            systemctl start stat-client
        ;;
        restart)
            systemctl restart stat-client
        ;;
        *)
           sshelp
        ;;
  esac
}

# 启用服务
function enable_server() {
    write_server
    systemctl enable stat-server
    systemctl start stat-server
    check_server
    if [[ -n ${SPID} ]]; then
        echo -e "${Info} Status Server 启动成功！"
    else
        echo -e "${Error} Status Server 启动失败！"
    fi
}
function enable_client() {
    write_client
    systemctl enable stat-client
    systemctl start stat-client
    check_client
    if [[ -n ${CPID} ]]; then
        echo -e "${Info} Status Client 启动成功！"
    else
        echo -e "${Error} Status Client 启动失败！"
    fi
}

function restart_client() {
    systemctl daemon-reload
    systemctl restart stat-client
    check_client
    if [[ -n ${CPID} ]]; then
        echo -e "${Info} Status Client 启动成功！"
    else
        echo -e "${Error} Status Client 启动失败！"
    fi
}


# 获取二进制文件
function get_status() {
    install_tool
    rm ServerStatus-${arch}-unknown-linux-musl.zip stat_*
    cd /tmp && wget "https://github.com/zdz/Serverstatus-Rust/releases/latest/download/ServerStatus-${arch}-unknown-linux-musl.zip"
    unzip -o ServerStatus-${arch}-unknown-linux-musl.zip
}

# 安装服务
function install_server() {
    echo -e "${Info} 下载 ${arch} 二进制文件"
    [ -f "/tmp/stat_server" ] || get_status
    mkdir -p ${server_dir}
    mv /tmp/stat_server /usr/local/ServerStatus/server/stat-server
    mv /tmp/config.toml /usr/local/ServerStatus/server/config.toml
    chmod +x /usr/local/ServerStatus/server/stat-server
    enable_server
}
function install_client() {
    echo -e "${Info} 下载 ${arch} 二进制文件"
    [ -f "/tmp/stat_client" ] || get_status
    mkdir -p ${client_dir}
    mv /tmp/stat_client /usr/local/ServerStatus/client/stat-client
    chmod +x /usr/local/ServerStatus/client/stat-client
    input_upm
    get_conf
    enable_client
}

function reset_conf() {
    if [ ! "$#" = 0 ]; then
        UPM="$1"; shift
        get_conf
        write_client
        restart_client
    else
        input_upm
        get_conf
        write_client
        restart_client
    fi  
}

# 卸载服务
function uninstall_server() {
    echo -e "${Error} 开始卸载 Server"
    systemctl stop stat-server
    systemctl disable stat-server
    rm -rf /usr/local/ServerStatus/server/
    rm -rf /usr/lib/systemd/system/stat-server.service
}
function uninstall_client() {
    echo -e "${Error} 开始卸载 Client"
    systemctl stop stat-client
    systemctl disable stat-client
    rm -rf /usr/local/ServerStatus/client/
    rm -rf /usr/lib/systemd/system/stat-client.service
}

function ssinstall() {
    INCMD="$1"; shift
    case ${INCMD} in
        --server|-s)
            install_server
        ;;
        --client|-c)
            if [ ! "$#" = 0 ]; then
                echo -e "${Info} 下载 ${arch} 二进制文件"
                [ -f "/tmp/stat_client" ] || get_status
                mv /tmp/stat_client /usr/local/ServerStatus/client/stat-client
                chmod +x /usr/local/ServerStatus/client/stat-client
                UPM="$1"; shift
                get_conf
                enable_client
            else
                install_client
            fi  
        ;;
        *)
            sshelp
        ;;
  esac
}

function ssuninstall() {
    INCMD="$1"; shift
    case ${INCMD} in
        --server|-s)
            uninstall_server
        ;;
        --client|-c)
            uninstall_client
        ;;
        *)
            sshelp
        ;;
  esac
}

if [ ! "$#" = 0 ]; then
    INCMD="$1"; shift
fi

case ${INCMD} in
    --install|-i)
        ssinstall "$@"
    ;;
    --uninstall|-uni|-u)
        ssuninstall "$@"
    ;;
    --reset|-r)
        reset_conf "$@"
    ;;
    --server|-s)
        ssserver "$@"
    ;;
    --client|-c)
        ssclient "$@"
    ;;
    --help|-h|*)
        sshelp
    ;;
esac
