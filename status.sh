#!/usr/bin/env bash
#=================================================
#  Description: Serverstat-Rust
#  Version: v1.0.2
#  Updater: Yooona-Lim
#  Update Description:
#         1.新增备份和恢复功能
#         2.新增版本号功能，供更新检查使用
#         3.可以同时卸载（适用于server和client在同一台机子的情况），实现很简单，两个实现的函数一起调用
#         4.地址字符串尽量用变量替代，方便修改
#=================================================

Info="\033[32m[信息]\033[0m"
Error="\033[31m[错误]\033[0m"
Tip="\033[32m[注意]\033[0m"

client_dir=/usr/local/ServerStatus/client/
server_dir=/usr/local/ServerStatus/server/

tmp_server_file=/tmp/stat_server
tmp_client_file=/tmp/stat_client

client_file=/usr/local/ServerStatus/client/stat_client
server_file=/usr/local/ServerStatus/server/stat_server
client_conf=/lib/systemd/system/stat_client.service
server_conf=/lib/systemd/system/stat_server.service
server_toml=/usr/local/ServerStatus/server/config.toml

bak_dir=/usr/local/ServerStatus/bak/

if [ "${MIRROR}" = CN ]; then
    echo cn
fi


function sshelp() {
    printf "
help:\n\
    -i,--install    安装 Status\n\
        -i -s           安装 Server\n\
        -i -c           安装 Client\n\
        -i -c conf      自动安装 Client\n\
    -up,--upgrade   升级 Status\n\
        -up -s          升级 Server\n\
        -up -c          升级 Client\n\
        -up -a          升级 Server and Client\n\
    -un,--uninstall  卸载 Status\n\
        -un -s           卸载 Server\n\
        -un -c           卸载 Client\n\
        -un -a           卸载 Server and Client\n\
    -r,--reset      更改 Status 配置\n\
        -r          更改 Client 配置\n\
        -r conf         自动更改 Client配置\n\
    -s,--server     管理 Status 运行状态\n\
        -s {status|start|stop|restart}\n\
    -c,--client     管理 Client 运行状态\n\
        -c {status|start|stop|restart}\n\n\
    -b,--bakup      备份 Status\n\
        -b -s          备份 Server\n\
        -b -c          备份 Client\n\
        -b -a          备份 Server and Client\n\
若无法访问 Github: \n\
    CN=true bash status.sh args
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

#安装unzip和wget工具
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
    if [ "${PROTOCOL}" = "grpc" ]; then
        echo -e "${Info} 使用 grpc 连接"
        MASTER=$(echo "${UPM}" |awk -F "[@]" '{print $2}')
    else
        echo -e "${Info} 使用 http 连接"
        MASTER=$(echo "${UPM}" |awk -F "[@]" '{print $2}')/report
    fi
}

# 检查服务
function check_server() {
    SPID=$(pgrep -f "stat_server")
}
function check_client() {
    CPID=$(pgrep -f "stat_client")
}

# 获取仓库最新版本号,运行可获得 'v1.7.2' 这样的版本号
function get_latest_version() {
  api_url="https://api.github.com/repos/zdz/ServerStatus-Rust/releases/latest"
  local latest_version # 声明和赋值分开写，是编译器给的警告
  latest_version=$(wget -qO- "$api_url" | grep -Po '(?<="tag_name": ")[^"]*')
  echo "$latest_version"
}

# 获取本地.service配置中的版本号，以 #Version=X.X.X 的注释形式存在
# 接受的参数为 -s 服务端 或 -c 客户端
function get_current_version(){
    conf_location=$1
    # 如果是 -s 参数，就设置为服务端配置文件的路径，否则 -c 为客户端配置文件的路径
    if [ "$1" = "-s" ]; then
        conf_location=${server_conf}
    elif [ "$1" = "-c" ]; then
        conf_location=${client_conf}
    fi
    current_version=$(grep -Po '(?<=Version=\s)v[\d.]+' "$conf_location")
    echo "$current_version"
}

# 往配置文件抬头写入版本号
# 接受的参数为 .service 配置文件的路径，和版本号
function write_version(){
    conf_location=$1
    version=$2
    if grep -q "Version=" "$conf_location"; then # 如果已经存在 Version 字段，就替换
        sed -i "s/Version=.*/Version=${version}/" "$conf_location"
    else
        sed -i "1iVersion=${version}" "$conf_location" # 在第一行插入
    fi
}

# 写入 systemd 配置
function write_server() {
    local "$latest_version"
    latest_version=$(get_latest_version)
    echo -e "${Info} 写入systemd配置中"
    cat >${server_conf} <<-EOF
Vesion=${latest_version}
[Unit]
Description=ServerStatus-Rust Server
After=network.target

[Service]
#User=nobody
#Group=nobody
Environment="RUST_BACKTRACE=1"
WorkingDirectory=/usr/local/ServerStatus
ExecStart=/usr/local/ServerStatus/server/stat_server -c /usr/local/ServerStatus/server/config.toml
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure

[Install]
WantedBy=multi-user.target
EOF
}

function write_client() {
    local "$latest_version"
    latest_version=$(get_latest_version)
    echo -e "${Info} 写入systemd配置中"
    cat >${client_conf} <<-EOF
Vesion=${latest_version}
[Unit]
Description=Serverstat-Rust Client
After=network.target

[Service]
User=root
Group=root
Environment="RUST_BACKTRACE=1"
WorkingDirectory=/usr/local/ServerStatus
ExecStart=/usr/local/ServerStatus/client/stat_client -a "${PROTOCOL}://${MASTER}" -u ${USER} -p ${PASSWD}
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
        status) # 新增状态检查命令
            systemctl status stat_server
        ;;
        start)
            systemctl start stat_server
        ;;
        stop)
            systemctl stop stat_server
        ;;
        restart)
            systemctl restart stat_server
        ;;
        *)
            sshelp
        ;;
  esac
}
function ssclient() {
    INCMD="$1"; shift
    case ${INCMD} in
        status) # 新增状态检查命令
            systemctl status stat_server
        ;;
        start)
            systemctl start stat_client
        ;;
        stop)
            systemctl stop stat_client
        ;;
        restart)
            systemctl restart stat_client
        ;;
        *)
           sshelp
        ;;
  esac
}

# 启用服务
function enable_server() {
    write_server
    systemctl enable stat_server
    systemctl start stat_server
    check_server
    if [[ -n ${SPID} ]]; then
        echo -e "${Info} Status Server 启动成功！"
    else
        echo -e "${Error} Status Server 启动失败！"
    fi
}
function enable_client() {
    write_client
    systemctl enable stat_client
    systemctl start stat_client
    check_client
    if [[ -n ${CPID} ]]; then
        echo -e "${Info} Status Client 启动成功！"
    else
        echo -e "${Error} Status Client 启动失败！"
    fi
}

function restart_client() {
    systemctl daemon-reload
    systemctl restart stat_client
    check_client
    if [[ -n ${CPID} ]]; then
        echo -e "${Info} Status Client 启动成功！"
    else
        echo -e "${Error} Status Client 启动失败！"
    fi
}


# 获取二进制文件，现在可以选择下载server或者client，并添加文件下载校验
function get_status() {
    if [ "${CN}" = true ]; then
        MIRROR="https://gh-proxy.com/"
    fi
    install_tool
    rm -f ServerStatus-${arch}-unknown-linux-musl.zip stat_*
    cd /tmp || exit

    # 判断为空或者 "-a" "--all"，为空可以兼容前面的函数功能
    if [ -z "$1" ] || [ "$1" = "-a" ] || [ "$1" = "--all" ]; then
        wget "${MIRROR}https://github.com/zdz/Serverstatus-Rust/releases/latest/download/server-${arch}-unknown-linux-musl.zip"
        wget "${MIRROR}https://github.com/zdz/Serverstatus-Rust/releases/latest/download/client-${arch}-unknown-linux-musl.zip"
        unzip -o server-${arch}-unknown-linux-musl.zip
        unzip -o client-${arch}-unknown-linux-musl.zip
    elif [ "$1" = "-s" ] || [ "$1" = "--server" ]; then
        wget "${MIRROR}https://github.com/zdz/Serverstatus-Rust/releases/latest/download/server-${arch}-unknown-linux-musl.zip"
        unzip -o server-${arch}-unknown-linux-musl.zip
    elif [ "$1" = "-c" ] || [ "$1" = "--client" ]; then
        wget "${MIRROR}https://github.com/zdz/Serverstatus-Rust/releases/latest/download/client-${arch}-unknown-linux-musl.zip"
        unzip -o client-${arch}-unknown-linux-musl.zip
    else
        echo "无效的参数"
        exit 1
    fi

    # 验证文件是否成功解压
    if [ $? -eq 0 ]; then
        echo "文件下载和解压成功！"
    else
        echo "文件下载或解压失败！"
        exit 1
    fi
}


# 安装服务
function install_server() {
    echo -e "${Info} 下载 ${arch} 二进制文件"
    [ -f "/tmp/stat_server" ] || get_status -s
    mkdir -p ${server_dir}
    mv $tmp_server_file $server_file
    mv /tmp/config.toml $server_toml
    chmod +x $server_file
    enable_server
}
function install_client() {
    echo -e "${Info} 下载 ${arch} 二进制文件"
    [ -f "/tmp/stat_client" ] || get_status -c
    mkdir -p ${client_dir}
    mv $tmp_client_file $client_file
    chmod +x $client_file
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

# 升级服务，解耦合
# 调用示例 
# upgrade_operation 'Server' "$server_file" "$server_conf" '-s' 'stat_server'
# upgrade_operation 'Client' "$client_file" "$client_conf" '-c' 'stat_client'
function upgrade_operation(){
    local component=$1
    local component_file=$2
    local component_conf=$3
    local get_status_arg=$4
    local systemctl_service=$5
    local latest_version=$6 # 获取最新版本

    echo -e "${Info} 开始升级 $component"
    systemctl stop "$systemctl_service"

    current_version=$(get_current_version "$get_status_arg") # 获取当前版本
    if [ "$current_version" != "$latest_version" ]; then
        echo -e "${Info} 与仓库版本号不一致，或配置文件没有版本号，现获取 $component 二进制文件，并更新配置文件的版本号"
        get_status "$get_status_arg"
        mv "$tmp_server_file" "$component_file"
        chmod +x "$component_file"
        write_version "$component_conf" "$latest_version"
    else
        echo -e "${Info} 当前 $component 版本已是最新版本 $latest_version"
    fi

    systemctl start "$systemctl_service"
}

# 卸载服务
function uninstall_server() {
    echo -e "${Tip} 开始卸载 Server"
    systemctl stop stat_server
    systemctl disable stat_server
    rm -rf $server_dir
    rm -rf $server_conf
}
function uninstall_client() {
    echo -e "${Tip} 开始卸载 Client"
    systemctl stop stat_client
    systemctl disable stat_client
    rm -rf $client_dir
    rm -rf $client_conf
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
                [ -f "/tmp/stat_client" ] || get_status '-c'
                mv $tmp_client_file $client_file
                chmod +x $client_file
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
        --all|-a)
            uninstall_server
            uninstall_client
        ;;
        *)
            sshelp
        ;;
  esac
}

# 升级版本
function ssupgrade() {
    local latest_version
    latest_version=$(get_latest_version)

    echo "Latest version of ServerStatus-Rust: $latest_version"

    INCMD="$1"; shift
    case ${INCMD} in
        --server|-s)
            upgrade_operation 'Server' "$server_file" "$server_conf" '-s' 'stat_server' "$latest_version"
        ;;
        --client|-c)
            upgrade_operation 'Client' "$client_file" "$client_conf" '-c' 'stat_client' "$latest_version"
        ;;
        --all|-a)
            upgrade_operation 'Server' "$server_file" "$server_conf" '-s' 'stat_server' "$latest_version"
            upgrade_operation 'Client' "$client_file" "$client_conf" '-c' 'stat_client' "$latest_version"
        ;;
        *)
            sshelp
        ;;
  esac
}

# 备份服务
function ssbakup() {
    INCMD="$1"; shift
    case ${INCMD} in
        --server|-s)
            echo -e "${Info} 开始备份 Server"
            systemctl stop stat_server
            
            cp $server_file $bak_dir
            cp $server_toml $bak_dir
            cp $server_conf $bak_dir

            systemctl start stat_server
            echo -e "${Info} 备份 Server 完成，文件路径：$bak_dir"
        ;;
        --client|-c)
            echo -e "${Info} 开始备份 Client"
            systemctl stop stat_client

            cp $client_file $bak_dir
            cp $client_conf $bak_dir

            systemctl start stat_client
            echo -e "${Info} 备份 Client 完成，文件路径：$bak_dir"
        ;;
        --all|-a)
            echo -e "${Info} 开始备份 Server"
            systemctl stop stat_server
            
            cp $server_file $bak_dir
            cp $server_toml $bak_dir
            cp $server_conf $bak_dir

            systemctl start stat_server
            echo -e "${Info} 备份 Server 完成，文件路径：$bak_dir"

            echo -e "${Info} 开始备份 Client"
            systemctl stop stat_client

            cp $client_file $bak_dir
            cp $client_conf $bak_dir

            systemctl start stat_client
            echo -e "${Info} 备份 Client 完成，文件路径：$bak_dir"
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
    --upgrade|-up)
        ssupgrade "$@"
    ;;
    --uninstall|-un)
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
    --bakup|-b)
        ssbakup "$@"
    ;;
    --help|-h|*)
        sshelp
    ;;
esac
