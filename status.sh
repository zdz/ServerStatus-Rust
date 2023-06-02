#!/usr/bin/env bash
#=================================================
#  Description: Serverstat-Rust
#  Version: v1.0.3
#  Updater: Yooona-Lim
#  Update Description:
#         1.新增恢复功能
#         2.改写短语
#=================================================

Info="\033[32m[信息]\033[0m"
Error="\033[31m[错误]\033[0m"
Warning="\033[33m[警告]\033[0m"
Tip="\033[32m[注意]\033[0m"

working_dir=/opt/ServerStatus

client_dir="$working_dir/client"
server_dir="$working_dir/server"

tmp_server_file=/tmp/stat_server
tmp_client_file=/tmp/stat_client

client_file="$client_dir/stat_client"
server_file="$server_dir/stat_server"
client_conf=/etc/systemd/system/stat_client.service
server_conf=/etc/systemd/system/stat_server.service
server_toml="$server_dir/config.toml"

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
    -rc,--reconfig      更改 Status 配置\n\
        -rc          更改 Client 配置\n\
        -rc conf         自动更改 Client配置\n\
    -s,--server     管理 Status 运行状态\n\
        -s {status|start|stop|restart}\n\
    -c,--client     管理 Client 运行状态\n\
        -c {status|start|stop|restart}\n\n\
    -b,--bakup      备份 Status\n\
        -b -s          备份 Server\n\
        -b -c          备份 Client\n\
        -b -a          备份 Server and Client\n\
    -rs,--restore    恢复 Status\n\
        -rs -s          恢复 Server\n\
        -rs -c          恢复 Client\n\
        -rs -a          恢复 Server and Client\n\n\
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

#先检查unzip和wget包是否存在，如果没有则安装unzip和wget工具
function install_tool() {
  if ! command -v unzip &> /dev/null; then
    echo "unzip not found. Installing unzip..."
    if [[ ${release} == "rpm" ]]; then
      yum -y install unzip
    elif [[ ${release} == "deb" ]]; then
      apt -y update
      apt -y install unzip
    fi
  fi

  if ! command -v wget &> /dev/null; then
    echo "wget not found. Installing wget..."
    if [[ ${release} == "rpm" ]]; then
      yum -y install wget
    elif [[ ${release} == "deb" ]]; then
      apt -y update
      apt -y install wget
    fi
  fi
}


# 获取服务端信息
function input_upm() {
    echo -e "${Tip} 请输入服务端的信息, 格式为 \"protocol://username:password@master:port\" (如输入错误 可以重新运行写入配置)  示例：\"http://h1:p1@127.0.0.1:8080\""
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

# 获取本地.service配置中的版本号，以 #Version=vX.X.X 的注释形式存在
# 接受的参数为 -s 服务端 或 -c 客户端
function get_current_version(){
    conf_location=$1
    # 如果是 -s 参数，就设置为服务端配置文件的路径，否则 -c 为客户端配置文件的路径
    if [ "$1" = "-s" ]; then
        conf_location=${server_conf}
    elif [ "$1" = "-c" ]; then
        conf_location=${client_conf}
    fi
    current_version=$(grep -Po '(?<=Version=)v[\d.]+' "$conf_location")
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
    local $latest_version
    latest_version=$(get_latest_version)
    echo -e "${Info} 写入systemd配置中"
    cat >${server_conf} <<-EOF
#Version=${latest_version}
[Unit]
Description=ServerStatus-Rust Server
After=network.target

[Service]
#User=nobody
#Group=nobody
Environment="RUST_BACKTRACE=1"
WorkingDirectory=${working_dir}
ExecStart=$server_file -c $server_toml
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure

[Install]
WantedBy=multi-user.target
EOF
}

function write_client() {
    local $latest_version
    latest_version=$(get_latest_version)
    echo -e "${Info} 写入systemd配置中"
    cat >${client_conf} <<-EOF
#Version=${latest_version}
[Unit]
Description=Serverstat-Rust Client
After=network.target

[Service]
User=root
Group=root
Environment="RUST_BACKTRACE=1"
WorkingDirectory=${working_dir}
ExecStart=$client_file -a "${PROTOCOL}://${MASTER}" -u ${USER} -p ${PASSWD}
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
            systemctl status stat_client
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
        wget --no-check-certificate -q "${MIRROR}https://github.com/zdz/Serverstatus-Rust/releases/latest/download/server-${arch}-unknown-linux-musl.zip"
        wget --no-check-certificate -q "${MIRROR}https://github.com/zdz/Serverstatus-Rust/releases/latest/download/client-${arch}-unknown-linux-musl.zip"
        unzip -o server-${arch}-unknown-linux-musl.zip
        unzip -o client-${arch}-unknown-linux-musl.zip
    elif [ "$1" = "-s" ] || [ "$1" = "--server" ]; then
        wget --no-check-certificate -q "${MIRROR}https://github.com/zdz/Serverstatus-Rust/releases/latest/download/server-${arch}-unknown-linux-musl.zip"
        unzip -o server-${arch}-unknown-linux-musl.zip
    elif [ "$1" = "-c" ] || [ "$1" = "--client" ]; then
        wget --no-check-certificate -q "${MIRROR}https://github.com/zdz/Serverstatus-Rust/releases/latest/download/client-${arch}-unknown-linux-musl.zip"
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
# upgrade_operation 'Server' "$server_file" "$tmp_server_file" "$server_conf" '-s' 'stat_server' "$latest_version"
# upgrade_operation 'Client' "$client_file" "$tmp_client_file" "$client_conf" '-c' 'stat_client' "$latest_version"
function upgrade_operation(){
    local component=$1 # 组件名
    local component_file=$2
    local temp_com_file=$3
    local component_conf=$4
    local get_status_arg=$5 
    local systemctl_service=$6 # systemctl 服务名
    local latest_version=$7 # 获取最新版本

    current_version=$(get_current_version "$get_status_arg") # 获取当前版本
    echo -e "${Info} 当前 $component 版本为 $current_version"
    if [ "$current_version" != "$latest_version" ]; then
        echo -e "${Info} $component 1.与仓库版本号不一致 2.或者配置文件没有版本号\n现获取 $component 二进制文件，并更新配置文件的版本号"
        echo -e "${Info} 开始升级 $component"
        get_status "$get_status_arg"

        systemctl stop "$systemctl_service"

        mv "$temp_com_file" "$component_file"
        chmod +x "$component_file"
        write_version "$component_conf" "$latest_version"
        systemctl daemon-reload

        systemctl restart "$systemctl_service"
    else
        echo -e "${Info} 当前 $component 版本已是最新版本 $latest_version"
    fi
}

# 卸载服务
function uninstall_server() {
    echo -e "${Tip} 开始卸载 Server"
    systemctl stop stat_server
    systemctl disable stat_server
    systemctl daemon-reload
    rm -rf $server_dir
    rm -rf $server_conf
}
function uninstall_client() {
    echo -e "${Tip} 开始卸载 Client"
    systemctl stop stat_client
    systemctl disable stat_client
    systemctl daemon-reload
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
            upgrade_operation 'Server' "$server_file" "$tmp_server_file" "$server_conf" '-s' 'stat_server' "$latest_version"
        ;;
        --client|-c)
            upgrade_operation 'Client' "$client_file" "$tmp_client_file" "$client_conf" '-c' 'stat_client' "$latest_version"
        ;;
        --all|-a)
            upgrade_operation 'Server' "$server_file" "$tmp_server_file" "$server_conf" '-s' 'stat_server' "$latest_version"
            upgrade_operation 'Client' "$client_file" "$tmp_client_file" "$client_conf" '-c' 'stat_client' "$latest_version"
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
            
            mkdir -p $bak_dir

            cp $server_file $bak_dir
            cp $server_toml $bak_dir
            cp $server_conf $bak_dir

            systemctl start stat_server
            echo -e "${Info} 备份 Server 完成，文件路径：$bak_dir"
        ;;
        --client|-c)
            echo -e "${Info} 开始备份 Client"
            systemctl stop stat_client

            mkdir -p $bak_dir

            cp $client_file $bak_dir
            cp $client_conf $bak_dir

            systemctl start stat_client
            echo -e "${Info} 备份 Client 完成，文件路径：$bak_dir"
        ;;
        --all|-a)
            echo -e "${Info} 开始备份 Server"
            systemctl stop stat_server
            
            mkdir -p $bak_dir

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

function restore_server(){
    echo -e "${Info} 开始恢复 Server"

    if [ -f "$server_file" ] || [ -f "$server_toml" ] || [ -f "$server_conf" ]; then
        echo -e "${Warning} 目标文件已存在，为避免可能存在的冲突，请删除以下文件后再恢复 Server: "
        [ -f "$server_file" ] && echo "$server_file"
        [ -f "$server_toml" ] && echo "$server_toml"
        [ -f "$server_conf" ] && echo "$server_conf"
        return
    fi

    mkdir -p $server_dir

    cp $bak_dir/stat_server $server_file # 实际上是 cp /usr/local/ServerStatus/bak/stat_server /opt/ServerStatus/server/stat_server
    cp $bak_dir/config.toml $server_toml
    cp $bak_dir/stat_server.service $server_conf # 实际上是 cp /usr/local/ServerStatus/bak/stat_server.service /etc/systemd/system/stat_server.service

    chmod +x $server_file
    systemctl enable stat_server
    systemctl start stat_server
    check_server
    if [[ -n ${SPID} ]]; then
        echo -e "${Info} Status Server 启动成功！"
    else
        echo -e "${Error} Status Server 启动失败！"
    fi

    echo -e "${Info} 恢复 Server 完成"
}

function restore_client(){
    echo -e "${Info} 开始恢复 Client"

    if [ -f "$client_file" ] || [ -f "$client_conf" ]; then
        echo -e "${Warning} 目标文件已存在，为避免可能存在的冲突，请删除以下文件后再恢复 Client: "
        [ -f "$client_file" ] && echo "$client_file"
        [ -f "$client_conf" ] && echo "$client_conf"
        return
    fi

    mkdir -p $client_dir

    cp $bak_dir/stat_client $client_file
    cp $bak_dir/stat_client.service $client_conf

    chmod +x $client_file
    systemctl enable stat_client
    systemctl start stat_client
    check_client
    if [[ -n ${CPID} ]]; then
        echo -e "${Info} Status Client 启动成功！"
    else
        echo -e "${Error} Status Client 启动失败！"
    fi

    echo -e "${Info} 恢复 Client 完成"
}

# 恢复服务
function ssrestore() {
    INCMD="$1"; shift
    case ${INCMD} in
        --server|-s)
            restore_server
        ;;
        --client|-c)
            restore_client
        ;;
        --all|-a)
            restore_server
            restore_client
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
    --reconfig|-rc)
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
    --restore|-rs)
        ssrestore "$@"
    ;;
    --help|-h|*)
        sshelp
    ;;
esac
