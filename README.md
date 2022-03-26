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
  - [7.感谢](#7感谢)

### 1.介绍
基于 `cppla/ServerStatus`，保持轻量和简化部署，特性如下：

- `rust` 版本 `server`, `client`，单个执行文件部署
- 支持上下线和简单自定义规则告警 (`telegram`, `wechat`)
- 支持 `vnstat` 统计月流量，重启不丢流量数据
- 支持 `tcp`, `http` 协议上报
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

# 不开启告警，可省略后面配置
# 告警间隔默认为30s
notify_interval = 30
# https://core.telegram.org/bots/api
# https://jinja.palletsprojects.com/en/3.0.x/templates/#if
[tgbot]
enabled = false
bot_token = "<tg bot token>"
chat_id = "<chat id>"
# host 可用字段参见 payload.rs 文件 HostStat 结构，模板置空则停用自定义告警，只保留上下线通知
custom_tpl = """
{% if host.memory_used / host.memory_total > 0.5  %}
<pre>❗{{ host.name }} 主机内存使用率超50%, 当前{{ (100 * host.memory_used / host.memory_total) | round }}%  </pre>
{% endif %}

{% if host.hdd_used / host.hdd_total  > 0.5  %}
<pre>❗{{ host.name }} 主机硬盘使用率超50%, 当前{{ (100 * host.hdd_used / host.hdd_total) | round }}% </pre>
{% endif %}
"""

[wechat]
enabled = false
corp_id = "<corp id>"
corp_secret = "<corp secret>"
agent_id = "<agent id>"
custom_tpl = """
{% if host.memory_used / host.memory_total > 0.8  %}
❗{{ host.name }} 主机内存使用率超80%
{% endif %}

{% if host.hdd_used / host.hdd_total  > 0.8  %}
❗{{ host.name }} 主机硬盘使用率超80%
{% endif %}
"""
```

### 3.2 服务端运行
```bash
# systemd 方式， 参照 one-touch.sh 脚本 (推荐)
systemctl enable stat_server
systemctl start stat_server

# docker
wget --no-check-certificate -qO docker-compose.yml 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/docker-compose.yml'
wget --no-check-certificate -qO config.toml 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/config.toml'
touch stats.json
docker network create traefik_gw
# 默认使用 watchtower 自动更新，不需要可以去掉
docker-compose up -d

# 源码编译
yum install -y openssl-devel
cargo build --release

# 运行
./stat_server
# 或
./stat_server -c config.toml
# 或
RUST_BACKTRACE=1 RUST_LOG=trace ./stat_server -c config.toml
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

## 运行
wget --no-check-certificate -qO client-linux.py 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/client/client-linux.py'
python3 client-linux.py -h
python3 client-linux.py -a "tcp://127.0.0.1:34512" -u h1 -p p1
# 或
python3 client-linux.py -a "http://127.0.0.1:8080/report" -u h1 -p p1

```


## 5.开启 `vnstat` 支持
[vnstat](https://zh.wikipedia.org/wiki/VnStat) 是Linux下一个流量统计工具，开启 `vnstat` 后，`server` 完全依赖客户机的 `vnstat` 数据来显示月流量和总流量，优点是重启不丢流量数据。
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

## 6.FAQ

<details>
  <summary>使用自定义修改主题</summary>

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
    root   /opt/ServerStatus/web; # web文件目录
    index  index.html index.htm;
  }
}
```

</details>


## 7.感谢
- https://github.com/cppla/ServerStatus
- https://github.com/BotoX/ServerStatus

