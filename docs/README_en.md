<p align="center">
  <a href="https://github.com/zdz/ServerStatus-Rust">
    <h1 align="center">✨ ServerStatus Cloud Probe in Rust<br>✨ Rust 版 ServerStatus 云探针</h1>
  </a>
</p>

<div align="center">
    <p>
        <a href="https://github.com/zdz/ServerStatus-Rust/actions/workflows/docker.yml">
            <img src="https://github.com/zdz/ServerStatus-Rust/actions/workflows/docker.yml/badge.svg"
                  alt="Docker">
        </a>
        <a href="https://github.com/zdz/ServerStatus-Rust/actions/workflows/release.yml">
            <img src="https://github.com/zdz/ServerStatus-Rust/actions/workflows/release.yml/badge.svg" alt="Release"></a>
        <a href="https://github.com/zdz/ServerStatus-Rust/issues">
            <img src="https://img.shields.io/github/issues/zdz/ServerStatus-Rust"
                  alt="GitHub issues">
        </a>
        <a href="https://github.com/zdz/ServerStatus-Rust/discussions">
            <img src="https://img.shields.io/github/discussions/zdz/ServerStatus-Rust"
                  alt="GitHub Discussions">
        </a>
        <a href="https://github.com/zdz/ServerStatus-Rust/releases">
            <img src="https://img.shields.io/github/v/release/zdz/ServerStatus-Rust"
                  alt="GitHub release (latest SemVer)">
        </a>
        <a href="https://github.com/zdz/ServerStatus-Rust/releases">
            <img src="https://img.shields.io/github/downloads/zdz/ServerStatus-Rust/total" alt="GitHub all releases">
        </a>
    </p>
</div>

<img width="1317" alt="image" src="https://user-images.githubusercontent.com/152173/206825541-6eaeb856-0c03-479a-b07e-006b60b41c02.png">
<img width="1436" alt="image" src="https://user-images.githubusercontent.com/152173/165958225-25fc8fda-5798-42f8-bac5-72d778c0bab5.png">

<br><br><br>
<h3 align="center">
<a href="../README.md">简体中文</a>
|
<a href="#">English</a>
<details>
  <summary>Translate detials</summary>
  <a href="https://github.com/mobeicanyue">mobeicanyue</a> Translated with the help of ChatGPT and DeepL
</details>
</h3>
<br><br>

<h2>Table of Contents</h2>

- [1. Introduction](#1-introduction)
  - [🍀 Theme](#-theme)
- [2. Installation and Deployment](#2-installation-and-deployment)
  - [2.1 Quick Experience](#21-quick-experience)
  - [2.2 Quick Deployment](#22-quick-deployment)
  - [2.3 Service Management Script Deployment](#23-service-management-script-deployment)
  - [2.4 Railway Deployment](#24-railway-deployment)
- [3. Server Configuration](#3-server-configuration)
  - [3.1 Configuration File config.toml](#31-configuration-file-configtoml)
  - [3.2 Running the server](#32-running-the-server)
- [4. Client Doucumentation](#4-client-doucumentation)
  - [4.1 Rust version Client](#41-rust-version-client)
  - [4.2 Python version Client](#42-python-version-client)
- [5. Enabling vnstat Support](#5-enabling-vnstat-support)
- [6. FAQ](#6-faq)
- [7. Related Projects](#7-related-projects)
- [8. Final thoughts](#8-final-thoughts)
- [9. Stargazers over time](#9-stargazers-over-time)

## 1. Introduction
  `ServerStatus` Power-Up Edition maintains lightweight and simple deployment while adding the following key features:

- Completely rewritten in `Rust`, both the server and client, allowing for deployment as a single executable file.
- Multiple system support including `Linux`, `MacOS`, `Windows`, `Android`, and `Raspberry Pi`.
- Support for online/offline monitoring and simple custom rule alerts via `Telegram`, `WeChat`, `email`, and `webhook`.
- Support for reporting via the `HTTP` protocol, making it easy to deploy on various free container services and optimize reporting routes with `Cloudflare` and other services.
- Support for monthly traffic statistics with `vnstat`, ensuring data is not lost during restarts.
- Support for quick deployment with `Railway`.
- Support for automatic startup with `systemd`.
- Other features,such as 🗺️,can be found in the [wiki](https://github.com/zdz/ServerStatus-Rust/wiki)

Demo: [ssr.rs](https://ssr.rs) | [cn dns](https://ck.ssr.rs)
|
Download: [Releases](https://github.com/zdz/ServerStatus-Rust/releases)
|
[Changelog](https://github.com/zdz/ServerStatus-Rust/releases)
|
Feedback: [Discussions](https://github.com/zdz/ServerStatus-Rust/discussions)

📚 Complete documentation has been migrated to [doc.ssr.rs](https://doc.ssr.rs)

### 🍀 Theme

If you think the theme you have created/modified is good, feel free to share/submit a pull request (PR). For frontend deployment instructions, please refer to the following steps. [#37](https://github.com/zdz/ServerStatus-Rust/discussions/37)

<details>
  <summary>Hotaru Theme</summary>

Hotaru theme modified and provided by [@HinataKato](https://github.com/HinataKato), [theme address](https://github.com/HinataKato/hotaru_theme_for_RustVersion)

<img width="1202" alt="image" src="https://user-images.githubusercontent.com/152173/167900971-5ef0c23a-af43-4f52-aab5-d58e4a66c8ea.png">

</details>

<details>
  <summary>ServerStatus-web Theme</summary>

ServerStatus-web theme modified and provided by [@mjjrock](https://github.com/mjjrock), [theme address](https://github.com/mjjrock/ServerStatus-web)

<img width="1425" alt="image" src="https://user-images.githubusercontent.com/102237118/171837653-3a5b2cd6-bf02-4602-a132-2c80a6707f68.png">

</details>


<details>
  <summary>Theme of v1.5.7 </summary>

[Demo](https://tz-rust.vercel.app)

<img width="1215" alt="image" src="https://user-images.githubusercontent.com/152173/165957689-d35714a9-f7f8-49f7-9573-97d4cf3c2f79.png">
</details>

## 2. Installation and Deployment

### 2.1 Quick Experience
```bash
# for CentOS/Debian/Ubuntu x86_64
mkdir -p /opt/ServerStatus && cd /opt/ServerStatus
# apt install -y unzip / yum install -y unzip
wget --no-check-certificate -qO one-touch.sh 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/one-touch.sh'
bash -ex one-touch.sh
# Once deployed, open http://127.0.0.1:8080/ or http://<your IP>:8080/
# Custom deployments can be found in the one-touch.sh script
```

### 2.2 Quick Deployment

👉 [Quick Deployment](https://doc.ssr.rs/rapid_deploy)

### 2.3 Service Management Script Deployment

[@Colsro](https://github.com/Colsro) Provide 

[@mobeicanyue](https://github.com/mobeicanyue) Maintain

> If the domain `raw.githubusercontent.com` is inaccessible, you can use the alternative address `cdn.jsdelivr.net`.

  - [https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/status.sh](https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/status.sh)
  - [https://cdn.jsdelivr.net/gh/zdz/ServerStatus-Rust@master/status.sh](https://cdn.jsdelivr.net/gh/zdz/ServerStatus-Rust@master/status.sh)

```bash
# download script
wget --no-check-certificate -qO status.sh 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/status.sh'
# ！！！Please make sure to execute it with root privileges.！！！

# install server
bash status.sh -i -s

# install client
bash status.sh -i -c
# or
bash status.sh -i -c protocol://username:password@master:port
# eg:
bash status.sh -i -c grpc://h1:p1@127.0.0.1:9394
bash status.sh -i -c http://h1:p1@127.0.0.1:8080

# More use cases
❯ bash status.sh

help:
    -i,--install    install Status
        -i -s           install Server
        -i -c           install Client
        -i -c conf      auto-install Client
    -up,--upgrade   upgrade Status
        -up -s          upgrade Server
        -up -c          upgrade Client
        -up -a          upgrade Server and Client
    -un,--uninstall  uninstall Status
        -un -s           uninstall Server
        -un -c           uninstall Client
        -un -a           uninstall Server and Client
    -rc,--reconfig      reconfig Status
        -rc          reconfig Client
        -rc conf         Auto reconfig Client
    -s,--server      manage the running status of Server
        -s {status|start|stop|restart}
    -c,--client      manage the running status of Client
        -c {status|start|stop|restart}
    -b,--bakup       backup Status
        -b -s          Backup Server
        -b -c          Backup Client
        -b -a          Backup Server and Client
    -rs,--restore    restore Status
        -rs -s          restore Server
        -rs -c          restore Client
        -rs -a          restore Server and Client
    -h,--help       for help
if you are in China Mainland, you can use the following command to speed up the download:
    CN=true bash status.sh args
```


### 2.4 Railway Deployment

Too lazy to configure `Nginx`, `SSL` certificates? Try
[Deploying Server in Railway](https://github.com/zdz/ServerStatus-Rust/wiki/Railway)

[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/new/template/kzT46l?referralCode=pJYbdU)


## 3. Server Configuration

### 3.1 Configuration File config.toml
```toml
# Listening address, ipv6 using [::]:9394
grpc_addr = "0.0.0.0:9394"
http_addr = "0.0.0.0:8080"
# Default 30s no uplink judgment offline
offline_threshold = 30

# admin account, if not set it will be randomly generated by default, used to view /detail, /map
admin_user = ""
admin_pass = ""

# hosts and hosts_group can be configured in any of two modes
# name: host unique identifier, no duplicates, alias for display name
# notify = false, disable alerts for single machine alone, generally for poor network, frequent up and down
# monthstart = 1, when vnstat is not enabled, it indicates the day of the month when monthly traffic is counted
# disabled = true, disable on a single machine
# location Support national flag emoji https://emojixd.com/group/flags
# or country abbreviations, such as cn us, etc. All countries are in the directory web/static/flags
# custom labels labels = "os=centos;ndd=2022/11/25;spec=2C/4G/60G;"
# os labels are optional, use reported data if not filled, ndd(next due date) next renewal time, spec is host spec
# os available values centos debian ubuntu alpine pi arch windows linux
hosts = [
  {name = "h1", password = "p1", alias = "n1", location = "🏠", type = "kvm", labels = "os=arch;ndd=2022/11/25;spec=2C/4G/60G;"}.
  {name = "h2", password = "p2", alias = "n2", location = "🏢", type = "kvm", disabled = false}.
  {name = "h3", password = "p3", alias = "n3", location = "🏡", type = "kvm", monthstart = 1}.
  { name = "h4", password = "p4", alias = "n4", location = "cn", type = "kvm", notify = true, labels = "ndd=2022/11/25;spec=2C/4G/60G;"}.
]

# Dynamic registration mode, no longer need to do individual configuration for each host
# gid is the template group id, dynamic registration unique identification, can not be repeated
hosts_group = [
  # Can be grouped by country, region or use
  {gid = "g1", password = "pp", location = "🏠", type = "kvm", notify = true}.
  {gid = "g2", password = "pp", location = "🏢", type = "kvm", notify = true}.
  # For example not sending notifications can be done as a separate group
  {gid = "silent", password = "pp", location = "🏡", type = "kvm", notify = false}.
]
# Invalid data cleanup interval in dynamic registration mode, default 30s
group_gc = 30

# Do not enable alerting, you can ignore the latter configuration, or remove the unwanted notification method
# Alert interval is 30s by default
notify_interval = 30
# https://core.telegram.org/bots/api
# https://jinja.palletsprojects.com/en/3.0.x/templates/#if
[tgbot]
# switch true on
enabled = false
bot_token = "<tg bot token>"
chat_id = "<chat id>"
# Available fields for host are in the payload.rs file HostStat structure, {{host.xxx}} is a placeholder variable
# For example, host.name can be replaced by host.alias, and people can write notification messages according to their own preferences
# {{ip_info.query}} Host ip address, {{sys_info.host_name}} host hostname
title = "❗<b>Server Status</b>"
online_tpl = "{{config.title}} \n😆 {{host.location}} {{host.name}} host is back online"
offline_tpl = "{{config.title}} \n😱 {{host.location}} {{host.name}} The host is offline"
# If the custom template is empty, custom alarms are disabled and only online and offline notifications are reserved
custom_tpl = """
{% if host.memory_used/host.memory_total > 0.5%}
<pre>😲 {{host.name}} The host memory usage exceeds 50%. {{(100 * host.memory_used/host.memory_total) | round}}% </pre>
{% endif %}

{% if host.hdd_used/host.hdd_total > 0.5%}
<pre>😲 {{host.name}} The disk usage of the host exceeds 50%. {{(100 * host.hdd_used/host.hdd_total) | round}}% </pre>
{% endif %}
"" "

# wechat, email, webhook and other notification methods configuration details see config.toml
```

### 3.2 Running the server
```bash
# systemd, see the one-touch.sh script (recommended).

# 💪 Manual mode
# help
./stat_server -h
# Manual operation
./stat_server -c config.toml
# or
RUST_BACKTRACE=1 RUST_LOG=trace ./stat_server -c config.toml

# Test that the configuration file is valid
./stat_server -c config.toml -t
# Send a test message according to the configuration to verify that the notification is in effect
./stat_server -c config.toml --notify-test

# 🐳 docker mode
wget --no-check-certificate -qO docker-compose.yml  'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/docker-compose.yml'
wget --no-check-certificate -qO config.toml 'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/config.toml'
touch stats.json
docker-compose up -d
```

## 4. Client Doucumentation

<details>
  <summary>System version & Architecture</summary>

|  OS            | Release  |
|  ----          | ----     |
| Linux x86_64   | x86_64-unknown-linux-musl |
| Linux arm64    | aarch64-unknown-linux-musl |
| MacOS x86_64   | x86_64-apple-darwin |
| MacOS arm64    | aarch64-apple-darwin |
| Windows x86_64 | x86_64-pc-windows-msvc |
| Raspberry Pi   | armv7-unknown-linux-musleabihf |
| Android 64bit  | aarch64-linux-android |
| Android 32bit  | armv7-linux-androideabi |

</details>

### 4.1 Rust version Client
```bash
# public environment suggest headscale/nebula networking or https, use nginx for server ssl and custom location /report
# alpine linux requires the command apk add procps iproute2 coreutils
# If the Rust version of the client doesn't work on your system, switch to the 4.2 Python cross-platform version below

# systemd method, see the one-touch.sh script (recommended)

# 💪 The manual way
# Rust version Client
. /stat_client -h
. /stat_client -a "http://127.0.0.1:8080/report" -u h1 -p p1
# or
. /stat_client -a "grpc://127.0.0.1:9394" -u h1 -p p1

# rust client available parameters
./stat_client -h
OPTIONS:
    -6, --ipv6                   ipv6 only, default:false
    -a, --addr <ADDR>            [default: http://127.0.0.1:8080/report]
        --alias <ALIAS>          alias for host [default: unknown]
        --cm <CM_ADDR>           China Mobile probe addr [default: cm.tz.cloudcpp.com:80]
        --ct <CT_ADDR>           China Telecom probe addr [default: ct.tz.cloudcpp.com:80]
        --cu <CU_ADDR>           China Unicom probe addr [default: cu.tz.cloudcpp.com:80]
        --disable-extra          disable extra info report, default:false
        --disable-notify         disable notify, default:false
        --disable-ping           disable ping, default:false
        --disable-tupd           disable t/u/p/d, default:false
    -g, --gid <GID>              group id [default: ]
    -h, --help                   Print help information
        --ip-info                show ip info, default:false
        --ip-source <IP_SOURCE>  ip info source [env: SSR_IP_SOURCE=] [default: ip-api.com]
        --sys-info               show sys info, default:false
        --json                   use json protocol, default:false
        --location <LOCATION>    location [default: ]
    -n, --vnstat                 enable vnstat, default:false
        --vnstat-mr <VNSTAT_MR>  vnstat month rotate 1-28 [default: 1]
    -p, --pass <PASS>            password [default: p1]
    -t, --type <HOST_TYPE>       host type [default: ]
    -u, --user <USER>            username [default: h1]
    -V, --version                Print version information
    -w, --weight <WEIGHT>        weight for rank [default: 0]

# Some parameter descriptions
--ip-info # Exit immediately after displaying local ip information, currently using ip-api.com data
--ip-source # Specify the source of ip information, ip-api.com / ip.sb / ipapi.co / myip.la
--sys-info # Exit immediately after displaying local system information
--disable-extra # Do not report system information and IP information
--disable-ping # Disable triple-net delay and packet loss detection
--disable-tupd # do not report tcp/udp/process/thread count, reduce CPU usage
-w, --weight # sorting and scoring, fine tune to bring hosts forward, ignore if you don't have OCD
-g, --gid # dynamically registered group id
--alias # Display name of specified hosts in dynamic registration mode
# Total traffic, NIC traffic/network speed statistics
-i, --iface # When non-empty, only the specified network port is counted
-e, --exclude-iface # Exclude specified network port, default exclude "lo,docker,vnet,veth,vmbr,kube,br-"
```

### 4.2 Python version Client

<details>
  <summary> Client for Python Description</summary>

```bash
# Python version Client depends on installation
## Centos
yum -y install epel-release
yum -y install python3-pip gcc python3-devel
python3 -m pip install psutil requests py-cpuinfo

## Ubuntu/Debian
apt -y install python3-pip
python3 -m pip install psutil requests py-cpuinfo

## Alpine linux
apk add wget python3 py3-pip gcc python3-dev musl-dev linux-headers
apk add procps iproute2 coreutils
python3 -m pip install psutil requests py-cpuinfo

wget --no-check-certificate -qO stat_client.py  'https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/client/stat_client.py'

## Windows
# Install python version 3.10 and set the environment variables
# The command line executes pip install psutil requests
# download https://raw.githubusercontent.com/zdz/ServerStatus-Rust/master/client/stat_client.py
pip install psutil requests py-cpuinfo

python3 stat_client.py -h
Python3 stat_client. Py - a "http://127.0.0.1:8080/report" -u h1 - p p1
```
</details>

## 5. Enabling vnstat Support
[vnstat](https://zh.wikipedia.org/wiki/VnStat) is a traffic statistics tool for Linux. When `vnstat` is enabled, the `server` relies entirely on the client's `vnstat` data to display monthly and total traffic, with the advantage that no traffic data is lost on reboot.

<details>
  <summary>Turn on the vnstat setting</summary>

```bash
# Install vnstat on client
## Centos
sudo yum install epel-release -y
sudo yum install -y vnstat
## Ubuntu/Debian
sudo apt install -y vnstat

Change /etc/vnstat.conf
# BandwidthDetection 0
# MaxBandwidth 0
# The default is not eth0. You need to empty the Interface to automatically select the network port
# No errors usually don't need to be corrected
# Interface ""
systemctl restart vnstat

# Make sure version >= 2.6
vnstat --version
# Test check monthly traffic (it may take a short time to collect data just after installation)
vnstat -m
vnstat --json m

# client Enable vnstat statistics with the -n parameter
/stat_client -a "grpc://127.0.0.1:9394" -u h1-p p1-n
# or
Python3 stat_client. Py - a "http://127.0.0.1:8080/report" -u h1 - p p1 - n
```
</details>

## 6. FAQ

<details>
  <summary>How to use custom themes</summary>

A simpler way 👉 [#37](https://github.com/zdz/ServerStatus-Rust/discussions/37)

```nginx
server {
  # ssl, domain, and other nginx configurations

  # Reverse /report requests
  location = /report {
    proxy_set_header Host $host.
    proxy_set_header X-Real-IP $remote_addr.
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for.
    proxy_set_header X-Forwarded-Proto $scheme.
    proxy_set_header X-Forwarded-Host $host.
    proxy_set_header X-Forwarded-Port $server_port.

    proxy_pass http://127.0.0.1:8080/report.
  }
  # Reverse json data requests
  location = /json/stats.json {
    proxy_set_header Host $host.
    proxy_set_header X-Real-IP $remote_addr.
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for.
    proxy_set_header X-Forwarded-Proto $scheme.
    proxy_set_header X-Forwarded-Host $host.
    proxy_set_header X-Forwarded-Port $server_port.

    proxy_pass http://127.0.0.1:8080/json/stats.json.
  }
  # After v1.4.0, you also need to inverse /detail, /map

  # Other html,js,css, etc., go to local text
  location / {
    root /opt/ServerStatus/web; # Your own modified theme directory
    index index.html index.htm.
  index index.html; }
index.html; }
```
</details>

<details>
  <summary>How to compile source code</summary>

```bash
# Follow the prompts to install the rust compiler
curl https://sh.rustup.rs -sSf | sh
yum install -y openssl-devel
git clone https://github.com/zdz/ServerStatus-Rust.git
cd ServerStatus-Rust
cargo build --release
# Compiled file directory target/release
```
</details>

<details>
  <summary>How to customize the ping address</summary>

```bash
# For example, customize the motion detection address by specifying the address with --cm
./stat_client -a "grpc://127.0.0.1:9394" -u h1 -p p1 --cm=cm.tz.cloudcpp.com:80

# Telecom Unicom parameters can be viewed with the -h command
./stat_client -h
OPTIONS:
    --cm <CM_ADDR>    China Mobile probe addr [default: cm.tz.cloudcpp.com:80]
    --ct <CT_ADDR>    China Telecom probe addr [default: ct.tz.cloudcpp.com:80]
    --cu <CU_ADDR>    China Unicom probe addr [default: cu.tz.cloudcpp.com:80]
```
</details>

<details>
  <summary>About this wheel</summary>

  I've been using `Prometheus` + `Grafana` + `Alertmanager` + `node_exporter` to do VPS monitoring, which is also a relatively mature monitoring solution in the industry, and after using it for a while, I found that in non-production environments, many monitoring indicators are not available, and the cost of operation and maintenance is a bit large.
   The `ServerStatus` is very good, simple and lightweight enough, you can see all the small machines at a glance, but the `c++` version has not been iterated for a long time, some of their own needs in the original version is not very good to modify, such as self-contained `tcp` reporting is not very friendly to cross-zone machines, but also not convenient to do optimization of the reported link and so on. This is a small project for learning `Rust`, so I won't add complicated features, keep it small and beautiful, simple to deploy, with [Uptime Kuma](https://github.com/louislam/uptime-kuma) basically can meet most of my monitoring needs.

</details>

## 7. Related Projects
- https://github.com/BotoX/ServerStatus
- https://github.com/cppla/ServerStatus
- https://github.com/mojeda/ServerStatus
- https://github.com/cokemine/ServerStatus-Hotaru
- https://github.com/ToyoDAdoubiBackup/ServerStatus-Toyo

## 8. Final thoughts

    I'm glad my code can run on your server. If it has been helpful to you, feel free to leave a star ⭐️ to show your support.
<br>

## 9. Stargazers over time
[![Stargazers over time](https://starchart.cc/zdz/ServerStatus-Rust.svg)](https://starchart.cc/zdz/ServerStatus-Rust)