# Rust 版 ServerStatus 云探针

[![Docker](https://github.com/zdz/ServerStatus-Rust/actions/workflows/docker.yml/badge.svg)](https://github.com/zdz/ServerStatus-Rust/actions/workflows/docker.yml)
[![Release](https://github.com/zdz/ServerStatus-Rust/actions/workflows/release.yml/badge.svg)](https://github.com/zdz/ServerStatus-Rust/actions/workflows/release.yml)
[![GitHub release (latest SemVer)](https://img.shields.io/github/v/release/zdz/ServerStatus-Rust)](https://github.com/zdz/ServerStatus-Rust/releases)
[![GitHub all releases](https://img.shields.io/github/downloads/zdz/ServerStatus-Rust/total)](https://github.com/zdz/ServerStatus-Rust/releases)

- [Rust 版 ServerStatus 云探针](#rust-版-serverstatus-云探针)
    - [1.介绍](#1介绍)
      - [演示：https://tz-rust.vercel.app](#演示httpstz-rustvercelapp)
      - [下载：Releases](#下载releases)
      - [反馈：Discussions](#反馈discussions)
  - [2.快速部署](#2快速部署)
  - [3.服务端说明](#3服务端说明)
    - [3.1 配置文件 `config.toml`](#31-配置文件-configtoml)
    - [3.2 服务端运行](#32-服务端运行)
  - [4.客户端说明](#4客户端说明)
  - [5.开启 `vnstat` 支持](#5开启-vnstat-支持)
  - [6.FAQ](#6faq)
  - [7.相关项目](#7相关项目)

### 1.介绍
基于 `cppla/ServerStatus`，保持轻量和简化部署，特性如下：

- 使用 `rust` 完全重写 `server`, `client`，单个执行文件部署
- 支持上下线和简单自定义规则告警 (`telegram`, `wechat`, `email`)
- 支持 `vnstat` 统计月流量，重启不丢流量数据
- 支持 `http` 协议上报，可配合 `CF` 等优化上报链路
- 支持 `railway` 一键部署
- 支持 `systemd`, 开机自启
- 更小 `docker` 镜像

#### 演示：https://tz-rust.vercel.app
#### 下载：[Releases](https://github.com/zdz/ServerStatus-Rust/releases)
#### 反馈：[Discussions](https://github.com/zdz/ServerStatus-Rust/discussions)

## 2.快速部署

```bash
# for x86_64
mkdir -p /opt/ServerStatus && cd /opt/ServerStatus
# apt install -y unzip / yum install -y unzip
wget --no-check-certificate -qO one-touch.sh 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/one-touch.sh'
bash -ex one-touch.sh
# 部署完毕，打开 http://127.0.0.1:8080/ 或 http://<你的IP>:8080/
# 自定义部署可参照 one-touch.sh 脚本
```

[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/new/template/kzT46l?referralCode=pJYbdU)

不想配置 `Nginx`，`SSL` ？了解一下
[Railway 部署 Server 教程](https://github.com/zdz/ServerStatus-Rust/wiki/Railway)

## 3.服务端说明

### 3.1 配置文件 `config.toml`
```toml
tcp_addr = "0.0.0.0:34512"
http_addr = "0.0.0.0:8080"
# 默认30s无上报判定下线
offline_threshold = 30

# 使用vnstat来更精准统计月流量，开启参考下面 vnstat 一节
vnstat = false

# name 主机唯一标识，不可重复，alias 为展示名
# 批量部署时可以用主机 hostname 作为 name，统一密码
hosts = [
  {name = "h1", password = "p1", alias = "n1", location = "🇨🇳", type = "kvm", monthstart = 1},
  {name = "h2", password = "p2", alias = "n2", location = "🇺🇸", type = "kvm", monthstart = 1},
]

# 不开启告警，可忽略后面配置
# 告警间隔默认为30s
notify_interval = 30
# https://core.telegram.org/bots/api
# https://jinja.palletsprojects.com/en/3.0.x/templates/#if
[tgbot]
enabled = false
bot_token = "<tg bot token>"
chat_id = "<chat id>"
# host 可用字段参见 payload.rs 文件 HostStat 结构, {{host.xxx}} 为占位变量
# 例如 host.name 可替换为 host.alias，自己根据喜好来编写通知消息
title = "❗<b>Server Status</b>"
online_tpl = "{{config.title}}  \n😆 {{host.location}} 的 {{host.name}} 主机恢复上线啦"
offline_tpl = "{{config.title}} \n😱 {{host.location}} 的 {{host.name}} 主机已经掉线啦"
# custom 模板置空则停用自定义告警，只保留上下线通知
custom_tpl = """
{% if host.memory_used / host.memory_total > 0.5  %}
<pre>😲{{ host.name }} 主机内存使用率超50%, 当前{{ (100 * host.memory_used / host.memory_total) | round }}%  </pre>
{% endif %}

{% if host.hdd_used / host.hdd_total  > 0.5  %}
<pre>😲{{ host.name }} 主机硬盘使用率超50%, 当前{{ (100 * host.hdd_used / host.hdd_total) | round }}% </pre>
{% endif %}
"""

# wechat, email 等其它通知方式 配置详细见 config.toml
```

### 3.2 服务端运行
```bash
# systemd 方式， 参照 one-touch.sh 脚本 (推荐)
systemctl enable stat_server
systemctl start stat_server

# 看看可用参数
./stat_server -h
# 手动运行
./stat_server
# 或
./stat_server -c config.toml
# 或
RUST_BACKTRACE=1 RUST_LOG=trace ./stat_server -c config.toml

# 测试配置文件是否有效
./stat_server -c config.toml -t
# 根据配置发送测试消息，验证通知是否生效
./stat_server -c config.toml --notify-test

# docker 方式
wget --no-check-certificate -qO docker-compose.yml 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/docker-compose.yml'
wget --no-check-certificate -qO config.toml 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/config.toml'
touch stats.json
docker network create traefik_gw
docker-compose up -d
```

## 4.客户端说明

```bash
# 公网环境建议 nebula 组网或走 https, 使用 nginx 对 server 套 ssl 和自定义 location /report

## systemd 方式， 参照 one-touch.sh 脚本 (推荐)
systemctl enable stat_client
systemctl start stat_client

# Rust 版本 Client
./stat_client -h
./stat_client -a "tcp://127.0.0.1:34512" -u h1 -p p1
# 或
./stat_client -a "http://127.0.0.1:8080/report" -u h1 -p p1

# Python 版本 Client 依赖安装
## Centos
sudo yum -y install epel-release
sudo yum -y install python3-pip gcc python3-devel
sudo python3 -m pip install psutil requests

## Ubuntu/Debian
sudo apt -y install python3-pip
sudo python3 -m pip install psutil requests

## 手动运行
wget --no-check-certificate -qO client-linux.py 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/client/client-linux.py'
python3 client-linux.py -h
python3 client-linux.py -a "tcp://127.0.0.1:34512" -u h1 -p p1
# 或
python3 client-linux.py -a "http://127.0.0.1:8080/report" -u h1 -p p1
```

## 5.开启 `vnstat` 支持
[vnstat](https://zh.wikipedia.org/wiki/VnStat) 是Linux下一个流量统计工具，开启 `vnstat` 后，`server` 完全依赖客户机的 `vnstat` 数据来显示月流量和总流量，优点是重启不丢流量数据。

<details>
  <summary>开启 vnstat 设置</summary>

```bash
# 在client端安装 vnstat
## Centos
sudo yum install epel-release -y
sudo yum install -y vnstat
## Ubuntu/Debian
sudo apt install -y vnstat

# 修改 /etc/vnstat.conf
# BandwidthDetection 0
# MaxBandwidth 0
# 默认不是 eth0 网口的需要置空 Interface 来自动选择网口
# 没报错一般不需要改
# Interface ""
systemctl restart vnstat

# 确保 version >= 2.6
vnstat --version
# 测试查看月流量 (刚安装可能需等一小段时间来采集数据)
vnstat -m
vnstat --json m

# server config.toml 开启 vnstat
vnstat = true

# client 使用 -n 参数开启 vnstat 统计
./stat_client -a "tcp://127.0.0.1:34512" -u h1 -p p1 -n
# 或
python3 client-linux.py -a "http://127.0.0.1:8080/report" -u h1 -p p1 -n
```
</details>

## 6.FAQ

<details>
  <summary>如何使用自定义主题</summary>

```nginx
server {
  # ssl,domain 等其它配置

  # 代理 /report 请求
  location = /report {
    proxy_set_header Host              $host;
    proxy_set_header X-Real-IP         $remote_addr;
    proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header X-Forwarded-Host  $host;
    proxy_set_header X-Forwarded-Port  $server_port;

    proxy_pass http://127.0.0.1:8080/report;
  }
  # 代理转发 json 数据请求
  location = /json/stats.json {
    proxy_set_header Host              $host;
    proxy_set_header X-Real-IP         $remote_addr;
    proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header X-Forwarded-Host  $host;
    proxy_set_header X-Forwarded-Port  $server_port;

    proxy_pass http://127.0.0.1:8080/json/stats.json;
  }

  # 其它 html,js,css 等，走本地文本
  location / {
    root   /opt/ServerStatus/web; # 你自己修改的主题目录
    index  index.html index.htm;
  }
}
```
</details>

<details>
  <summary>如何源码编译</summary>

```bash
# 按提示安装 rust 编译器
curl https://sh.rustup.rs -sSf | sh
yum install -y openssl-devel
git clone https://github.com/zdz/ServerStatus-Rust.git
cd ServerStatus-Rust
cargo build --release
# 编译好的文件目录 target/release
```
</details>

<details>
  <summary>如何自定义 ping 地址</summary>

```bash
# 例如自定义移动探测地址，用 --cm 指定地址
./stat_client -a "tcp://127.0.0.1:34512" -u h1 -p p1 --cm=cm.tz.cloudcpp.com:80

# 电信联通参数可以使用 -h 命令查看
./stat_client -h
# rust client 可用参数
OPTIONS:
    -a, --addr <ADDR>     [default: http://127.0.0.1:8080/report]
        --cm <CM_ADDR>    China Mobile probe addr [default: cm.tz.cloudcpp.com:80]
        --ct <CT_ADDR>    China Telecom probe addr [default: ct.tz.cloudcpp.com:80]
        --cu <CU_ADDR>    China Unicom probe addr [default: cu.tz.cloudcpp.com:80]
        --disable-ping    disable ping, default:false
        --disable-tupd    disable t/u/p/d, default:false
    -h, --help            Print help information
    -n, --vnstat          enable vnstat, default:false
    -p, --pass <PASS>     password [default: p1]
    -u, --user <USER>     username [default: h1]
    -V, --version         Print version information
```
</details>

## 7.相关项目
- https://github.com/cppla/ServerStatus
- https://github.com/BotoX/ServerStatus

