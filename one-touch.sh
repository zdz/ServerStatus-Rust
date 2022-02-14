#!/bin/bash
set -ex
WORKSPACE=/opt/ServerStatus

mkdir -p ${WORKSPACE}
cd ${WORKSPACE}
wget --no-check-certificate -qO ServerStatus-x86_64-unknown-linux-musl.zip  "https://github.com/zdz/ServerStatus-Rust/releases/download/latest/ServerStatus-x86_64-unknown-linux-musl.zip"
unzip -o ServerStatus-x86_64-unknown-linux-musl.zip

mv -v stat_server.service /etc/systemd/system/stat_server.service
mv -v stat_client.service /etc/systemd/system/stat_client.service

systemctl daemon-reload

systemctl start stat_server
systemctl start stat_client

systemctl status stat_server
systemctl status stat_client
