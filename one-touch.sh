#!/bin/bash
set -ex
WORKSPACE=/opt/ServerStatus

mkdir -p ${WORKSPACE}
wget --no-check-certificate -qO ServerStatus-x86_64-unknown-linux-musl.zip  "https://github.com/zdz/ServerStatus-Rust/releases/download/v1.1/ServerStatus-x86_64-unknown-linux-musl.zip"
unzip ServerStatus-x86_64-unknown-linux-musl.zip

cp stat_server.service /etc/systemd/system/stat_server.service
cp stat_client.service /etc/systemd/system/stat_client.service

systemctl daemon-reload

systemctl start stat_server
systemctl start stat_client

systemctl status stat_server
systemctl status stat_client
