#!/bin/bash
set -ex
WORKSPACE=/opt/ServerStatus

# 下载
mkdir -p ${WORKSPACE}
cd ${WORKSPACE}
wget --no-check-certificate -qO ServerStatus-x86_64-unknown-linux-musl.zip  "https://github.com/zdz/ServerStatus-Rust/releases/download/latest/ServerStatus-x86_64-unknown-linux-musl.zip"
unzip -o ServerStatus-x86_64-unknown-linux-musl.zip

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

# 修改 /etc/systemd/system/stat_client.service 文件，将IP改为你服务器的IP
