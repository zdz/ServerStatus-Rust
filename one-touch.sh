#!/bin/bash
set -ex

WORKSPACE=/opt/ServerStatus
mkdir -p ${WORKSPACE}
cd ${WORKSPACE}

# 下载, arm 机器替换 x86_64 为 aarch64
OS_ARCH="x86_64"
latest_version=$(curl -m 10 -sL "https://api.github.com/repos/zdz/ServerStatus-Rust/releases/latest" | grep "tag_name" | head -n 1 | awk -F ":" '{print $2}' | sed 's/\"//g;s/,//g;s/ //g')

wget --no-check-certificate -qO "server-${OS_ARCH}-unknown-linux-musl.zip"  "https://github.com/zdz/ServerStatus-Rust/releases/download/${latest_version}/server-${OS_ARCH}-unknown-linux-musl.zip"
wget --no-check-certificate -qO "client-${OS_ARCH}-unknown-linux-musl.zip"  "https://github.com/zdz/ServerStatus-Rust/releases/download/${latest_version}/client-${OS_ARCH}-unknown-linux-musl.zip"

unzip -o "server-${OS_ARCH}-unknown-linux-musl.zip"
unzip -o "client-${OS_ARCH}-unknown-linux-musl.zip"

# systemd service
mv -v stat_server.service /etc/systemd/system/stat_server.service
mv -v stat_client.service /etc/systemd/system/stat_client.service

systemctl daemon-reload

# 启动
systemctl start stat_server
systemctl start stat_client

# 状态查看
systemctl status stat_server
systemctl status stat_client

# 使用以下命令开机自启
# systemctl enable stat_server
# systemctl enable stat_client

# 停止
# systemctl stop stat_server
# systemctl stop stat_client

# https://fedoraproject.org/wiki/Systemd/zh-cn
# https://docs.fedoraproject.org/en-US/quick-docs/understanding-and-administering-systemd/index.html

# 修改 /etc/systemd/system/stat_client.service 文件，将IP改为你服务器的IP或你的域名
