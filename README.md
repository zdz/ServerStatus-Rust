# ServerStatus - Rust

[![Docker](https://github.com/zdz/ServerStatus-Rust/actions/workflows/docker.yml/badge.svg)](https://github.com/zdz/ServerStatus-Rust/actions/workflows/docker.yml)
[![Release](https://github.com/zdz/ServerStatus-Rust/actions/workflows/release.yml/badge.svg)](https://github.com/zdz/ServerStatus-Rust/actions/workflows/release.yml)

### 介绍
基于 `cppla` 版本 `ServerStatus`，特性如下：

- `rust` 版本 `server`, `client`，单个执行文件部署
- 支持上下线和简单自定义规则告警 (`telegram`, `wechat`)
- 支持 `vnstat` 更精准统计月流量
- 支持 `tcp`, `http` 协议上报
- 支持 `systemd`, 开机自启
- 更小 `docker` 镜像


### [Release下载](https://github.com/zdz/ServerStatus-Rust/releases)

## 快速安装
```bash
# for x86_64
mkdir -p /opt/ServerStatus && cd /opt/ServerStatus
# apt install -y unzip / yum install -y unzip
wget --no-check-certificate -qO one-touch.sh 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/one-touch.sh'
bash -ex one-touch.sh
# open http://127.0.0.1:8080/
```

## 服务端

配置文件 `config.toml`
```toml
tcp_addr = "0.0.0.0:34512"
http_addr = "0.0.0.0:8080"

# 使用vnstat来更精准统计月流量，开启参考下面 vnstat 一节
vnstat = false

# name 不可重复，代替原来的 ClientID
hosts = [
  {name = "h1", password = "p1", location = "🇨🇳", type = "kvm", monthstart = 1},
  {name = "h2", password = "p2", location = "us", type = "kvm", monthstart = 1},
]


# https://core.telegram.org/bots/api
# https://jinja.palletsprojects.com/en/3.0.x/templates/#if
[tgbot]
enabled = false
bot_token = "<tg bot token>"
chat_id = "<chat id>"
# host参见payload文件HostStat结构，模板置空则停用自定义告警
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


```bash
# docker
wget --no-check-certificate -qO docker-compose.yml 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/docker-compose.yml'
wget --no-check-certificate -qO config.toml 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/config.toml'
touch stats.json
docker-compose up -d

# 源码编译
yum install -y openssl-devel
cargo build --release

# 运行
./stat_server
或
./stat_server -c config.toml
或
RUST_BACKTRACE=1 RUST_LOG=trace ./stat_server -c config.toml

## systemd
systemctl enable stat_server
systemctl start stat_server

```

## 客户端
```bash
# Rust 版本 Client
./stat_client -h
./stat_client -a "tcp://127.0.0.1:34512" -u h1 -p p1
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
python3 client-linux.py -a "http://127.0.0.1:8080/report" -u h1 -p p1

## systemd
systemctl enable stat_client
systemctl start stat_client
```

## 开启 `vnstat` 支持
[vnstat](https://zh.wikipedia.org/wiki/VnStat) 是Linux下一个流量统计工具，开启 `vnstat` 后，`server` 完全依赖客户机的 `vnstat` 数据来显示月流量，优点是重启不丢流量数据，数据更准确。
```bash
# 在client端安装 vnstat
## Centos
sudo yum install epel-release -y
sudo yum install -y vnstat
## Ubuntu/Debian
sudo apt install -y vnstat

# 修改 /etc/vnstat.conf
# MaxBandwidth 0
systemctl restart vnstat

# 确保 version >=2.6
vnstat --version
# 测试查看月流量
vnstat -m
vnstat --json m

# server config.toml 开启 vnstat
vnstat = true

# client 使用 -n 参数开启 vnstat 统计
./stat_client -a "tcp://127.0.0.1:34512" -u h1 -p p1 -n
或
python3 client-linux.py -a "http://127.0.0.1:8080/report" -u h1 -p p1 -n
```


## 参考
- https://github.com/cppla/ServerStatus

