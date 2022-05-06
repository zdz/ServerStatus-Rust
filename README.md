# ✨ Rust 版 ServerStatus 云探针

[![Docker](https://github.com/zdz/ServerStatus-Rust/actions/workflows/docker.yml/badge.svg)](https://github.com/zdz/ServerStatus-Rust/actions/workflows/docker.yml)
[![Release](https://github.com/zdz/ServerStatus-Rust/actions/workflows/release.yml/badge.svg)](https://github.com/zdz/ServerStatus-Rust/actions/workflows/release.yml)
[![GitHub release (latest SemVer)](https://img.shields.io/github/v/release/zdz/ServerStatus-Rust)](https://github.com/zdz/ServerStatus-Rust/releases)
[![GitHub all releases](https://img.shields.io/github/downloads/zdz/ServerStatus-Rust/total)](https://github.com/zdz/ServerStatus-Rust/releases)

<img width="1215" alt="image" src="https://user-images.githubusercontent.com/152173/165957689-d35714a9-f7f8-49f7-9573-97d4cf3c2f79.png">
<img width="1436" alt="image" src="https://user-images.githubusercontent.com/152173/165958225-25fc8fda-5798-42f8-bac5-72d778c0bab5.png">

- [✨ Rust 版 ServerStatus 云探针](#-rust-版-serverstatus-云探针)
  - [1. 介绍](#1-介绍)
  - [2. 安装部署](#2-安装部署)
    - [2.1 快速体验](#21-快速体验)
    - [2.2 服务管理脚本部署，感谢 @Colsro 提供](#22-服务管理脚本部署感谢-colsro-提供)
    - [2.3 Railway 部署](#23-railway-部署)
  - [3. 服务端说明](#3-服务端说明)
    - [3.1 配置文件 `config.toml`](#31-配置文件-configtoml)
    - [3.2 服务端运行](#32-服务端运行)
  - [4. 客户端说明](#4-客户端说明)
    - [4.1 Linux (`CentOS`, `Ubuntu`, `Debian`)](#41-linux-centos-ubuntu-debian)
    - [4.2 跨平台版本 (`Window`, `Linux`, `...`)](#42-跨平台版本-window-linux-)
  - [5. 开启 `vnstat` 支持](#5-开启-vnstat-支持)
  - [6. FAQ](#6-faq)
  - [7. 相关项目](#7-相关项目)

## 1. 介绍
`cppla/ServerStatus` 威力加强版，保持轻量和简化部署，增加主要特性如下：

- 使用 `rust` 完全重写 `server`、`client`，单个执行文件部署
- 支持上下线和简单自定义规则告警 (`telegram`、 `wechat`、 `email`)
- 支持 `http` 协议上报，可配合 `cf` 等优化上报链路
- 支持 `vnstat` 统计月流量，重启不丢流量数据
- 支持 `railway` 快速部署
- 支持 `systemd` 开机自启
- 其它功能，如 🗺️  见 [wiki](https://github.com/zdz/ServerStatus-Rust/wiki)

演示：[tz-rust.vercel.app](https://tz-rust.vercel.app)
|
下载：[Releases](https://github.com/zdz/ServerStatus-Rust/releases)
|
反馈：[Discussions](https://github.com/zdz/ServerStatus-Rust/discussions)

## 2. 安装部署

### 2.1 快速体验
```bash
# for CentOS/Debian/Ubuntu x86_64
mkdir -p /opt/ServerStatus && cd /opt/ServerStatus
# apt install -y unzip / yum install -y unzip
wget --no-check-certificate -qO one-touch.sh 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/one-touch.sh'
bash -ex one-touch.sh
# 部署完毕，打开 http://127.0.0.1:8080/ 或 http://<你的IP>:8080/
# 自定义部署可参照 one-touch.sh 脚本
```

### 2.2 服务管理脚本部署，感谢 [@Colsro](https://github.com/Colsro) 提供
<details>
  <summary>管理脚本使用说明</summary>

```bash
# 下载脚本
wget --no-check-certificate -qO status.sh 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/status.sh'

# 安装 服务端
bash status.sh -i -s

# 安装 客户端
bash status.sh -i -c
# or
bash status.sh -i -c protocol://username:password@master:port
# eg:
bash status.sh -i -c grpc://h1:p1@127.0.0.1:9394
bash status.sh -i -c http://h1:p1@127.0.0.1:8080

# 更多用法：
❯ bash status.sh

help:
    -i,--install    安装 Status
        -i -s           安装 Server
        -i -c           安装 Client
        -i -c conf      自动安装 Client
    -u,--uninstall  卸载 Status
        -u -s           卸载 Server
        -u -c           卸载 Client
    -r,--reset      更改 Status 配置
        -r          更改 Client 配置
        -r conf         自动更改 Client配置
    -s,--server     管理 Status 运行状态
        -s {start|stop|restart}
    -c,--client     管理 Client 运行状态
        -c {start|stop|restart}

若无法访问 Github:
    CN=true bash status.sh args
# 可能有点用
```
</details>


### 2.3 Railway 部署

懒得配置 `Nginx`，`SSL` 证书？试试
[在 Railway 部署 Server 教程](https://github.com/zdz/ServerStatus-Rust/wiki/Railway)

[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/new/template/kzT46l?referralCode=pJYbdU)

## 3. 服务端说明

### 3.1 配置文件 `config.toml`
```toml
# 侦听地址, ipv6 使用 [::]:9394
grpc_addr = "0.0.0.0:9394"
http_addr = "0.0.0.0:8080"
# 默认30s无上报判定下线
offline_threshold = 30

# 管理员账号,不设置默认随机生成，用于查看 /detail, /map
admin_user = ""
admin_pass = ""

# name 主机唯一标识，不可重复，alias 为展示名
# 使用 ansible 批量部署时可以用主机 hostname 作为 name，统一密码
# notify = false 单独禁止单台机器的告警，一般针对网络差，频繁上下线
# monthstart = 1 没启用 vnstat 时，表示月流量从每月哪天开始统计
# disabled = true 单机禁用，跟删除这条配置的效果一样
hosts = [
  {name = "h1", password = "p1", alias = "n1", location = "🏠", type = "kvm"},
  {name = "h2", password = "p2", alias = "n2", location = "🏢", type = "kvm", notify = true},
  {name = "h3", password = "p3", alias = "n3", location = "🏝️", type = "kvm", monthstart = 1},
  {name = "h4", password = "p4", alias = "n4", location = "🏢", type = "kvm", disabled = false},
]

# 不开启告警，可忽略后面配置，或者删除不需要的通知方式
# 告警间隔默认为30s
notify_interval = 30
# https://core.telegram.org/bots/api
# https://jinja.palletsprojects.com/en/3.0.x/templates/#if
[tgbot]
enabled = false
bot_token = "<tg bot token>"
chat_id = "<chat id>"
# host 可用字段参见 payload.rs 文件 HostStat 结构, {{host.xxx}} 为占位变量
# 例如 host.name 可替换为 host.alias，大家根据喜好来编写通知消息
title = "❗<b>Server Status</b>"
online_tpl  = "{{config.title}} \n😆 {{host.location}} 的 {{host.name}} 主机恢复上线啦"
offline_tpl = "{{config.title}} \n😱 {{host.location}} 的 {{host.name}} 主机已经掉线啦"
# custom 模板置空则停用自定义告警，只保留上下线通知
custom_tpl = """
{% if host.memory_used / host.memory_total > 0.5  %}
<pre>😲 {{host.name}} 主机内存使用率超50%, 当前{{ (100 * host.memory_used / host.memory_total) | round }}%  </pre>
{% endif %}

{% if host.hdd_used / host.hdd_total  > 0.5  %}
<pre>😲 {{host.name}} 主机硬盘使用率超50%, 当前{{ (100 * host.hdd_used / host.hdd_total) | round }}% </pre>
{% endif %}
"""

# wechat, email 等其它通知方式 配置详细见 config.toml
```

### 3.2 服务端运行
```bash
# systemd 方式， 参照 one-touch.sh 脚本 (推荐)

# 手动方式
# help
./stat_server -h
# 手动运行
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

## 4. 客户端说明

### 4.1 Linux (`CentOS`, `Ubuntu`, `Debian`)
```bash
# 公网环境建议 nebula 组网或走 https, 使用 nginx 对 server 套 ssl 和自定义 location /report
# Rust 版只在 CentOS, Ubuntu, Debian 测试过
# 如果 Rust 版客户端在你的系统无法使用，请切换到下面 4.2 跨平台版本

# systemd 方式， 参照 one-touch.sh 脚本 (推荐)

# 手动方式
# Rust 版本 Client
./stat_client -h
./stat_client -a "http://127.0.0.1:8080/report" -u h1 -p p1
# 或
./stat_client -a "grpc://127.0.0.1:9394" -u h1 -p p1

# rust client 可用参数
./stat_client -h
OPTIONS:
    -6, --ipv6             ipv6 only, default:false
    -a, --addr <ADDR>      [default: http://127.0.0.1:8080/report]
        --cm <CM_ADDR>     China Mobile probe addr [default: cm.tz.cloudcpp.com:80]
        --ct <CT_ADDR>     China Telecom probe addr [default: ct.tz.cloudcpp.com:80]
        --cu <CU_ADDR>     China Unicom probe addr [default: cu.tz.cloudcpp.com:80]
        --disable-extra    disable extra info report, default:false
        --disable-ping     disable ping, default:false
        --disable-tupd     disable t/u/p/d, default:false
    -h, --help             Print help information
        --ip-info          show ip info, default:false
        --json             use json protocol, default:false
    -n, --vnstat           enable vnstat, default:false
    -p, --pass <PASS>      password [default: p1]
    -u, --user <USER>      username [default: h1]
    -V, --version          Print version information

# 一些参数说明
--ip-info       # 显示本机ip信息后立即退出，目前使用 ip-api.com 数据
--disable-extra # 不上报系统信息和IP信息
--disable-ping  # 停用三网延时和丢包率探测
--disable-tupd  # 不上报 tcp/udp/进程数/线程数，减少CPU占用
```

### 4.2 跨平台版本 (`Window`, `Linux`, `...`)

<details>
  <summary>跨平台版本说明</summary>

```bash
# Python 版本 Client 依赖安装
## Centos
yum -y install epel-release
yum -y install python3-pip gcc python3-devel
python3 -m pip install psutil requests py-cpuinfo

## Ubuntu/Debian
apt -y install python3-pip
python3 -m pip install psutil requests py-cpuinfo

## Alpine linux
apk add wget python3 py3-pip gcc python3-dev musl-dev linux-headers
python3 -m pip install psutil requests py-cpuinfo

wget --no-check-certificate -qO stat_client.py 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/client/stat_client.py'

## Windows
# 安装 python 3.10 版本，并设置环境变量
# 命令行执行 pip install psutil requests
# 下载 https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/client/stat_client.py
pip install psutil requests py-cpuinfo

python3 stat_client.py -h
python3 stat_client.py -a "http://127.0.0.1:8080/report" -u h1 -p p1
```
</details>

## 5. 开启 `vnstat` 支持
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
# 从 v1.3.6 不再需要在 server 配置开启，client 自由选择启用与否，client 可部分打开，部分关闭
vnstat = true

# client 使用 -n 参数开启 vnstat 统计
./stat_client -a "grpc://127.0.0.1:9394" -u h1 -p p1 -n
# 或
python3 stat_client.py -a "http://127.0.0.1:8080/report" -u h1 -p p1 -n
```
</details>

## 6. FAQ

<details>
  <summary>如何使用自定义主题</summary>

```nginx
server {
  # ssl,domain 等其它配置

  # 反代 /report 请求
  location = /report {
    proxy_set_header Host              $host;
    proxy_set_header X-Real-IP         $remote_addr;
    proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header X-Forwarded-Host  $host;
    proxy_set_header X-Forwarded-Port  $server_port;

    proxy_pass http://127.0.0.1:8080/report;
  }
  # 反代 json 数据请求
  location = /json/stats.json {
    proxy_set_header Host              $host;
    proxy_set_header X-Real-IP         $remote_addr;
    proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header X-Forwarded-Host  $host;
    proxy_set_header X-Forwarded-Port  $server_port;

    proxy_pass http://127.0.0.1:8080/json/stats.json;
  }
  # v1.4.0后，同样需要反代  /detail, /map

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
./stat_client -a "grpc://127.0.0.1:9394" -u h1 -p p1 --cm=cm.tz.cloudcpp.com:80

# 电信联通参数可以使用 -h 命令查看
./stat_client -h
OPTIONS:
    --cm <CM_ADDR>    China Mobile probe addr [default: cm.tz.cloudcpp.com:80]
    --ct <CT_ADDR>    China Telecom probe addr [default: ct.tz.cloudcpp.com:80]
    --cu <CU_ADDR>    China Unicom probe addr [default: cu.tz.cloudcpp.com:80]
```
</details>

<details>
  <summary>关于这个轮子</summary>

  之前一直在使用 `Prometheus` + `Grafana` + `Alertmanager` + `node_exporter` 做VPS监控，这也是业界比较成熟的监控方案，用过一段时间后，发现非生产环境的话，很多监控指标都用不上，反而显得有些重。
  `ServerStatus` 很好，足够简单和轻量，一眼可以看尽大好山河，只是 `c++` 版本很久没迭代过，自己的一些需求在原版上不是很好修改，如自带 `tcp` 上报对跨区机器不是很友好，也不方便对上报的链路优化 等等。过年的时候正值疫情闲来无事，学习 `Rust` 正好需要个小项目练手，于是撸了个 `ServerStatus` 来练手，项目后面会继续维护但不会增加复杂的功能，保持小而美，简单部署，配合 [Uptime Kuma](https://github.com/louislam/uptime-kuma) 基本上可以满足个人大部分监控需求。

</details>

## 7. 相关项目
- https://github.com/cppla/ServerStatus
- https://github.com/BotoX/ServerStatus
