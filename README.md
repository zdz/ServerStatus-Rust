# ServerStatus - Rust

## 介绍
项目基于`cppla`版本`ServerStatus`，修改如下：

- `rust`版本`server`，单个执行文件部署
- 支持简单自定义 `telegram`，`wechat` 规则告警
- 使用`http`协议上报
- 支持`systemd`, 开机自启
- 更小`docker`镜像(5M)

## 服务端

配置文件 `config.toml`
```toml
addr = "0.0.0.0:8080"
log_level = "trace"

# admin pass
admin_pass = "<admin pass>"
admin_user = "<admin name>"

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

# name 不可重复，代替原来的 ClientID/ClientNetID
[[hosts]]
name = "h1"
password = "p1"
host = "h1"
location = "🇨🇳"
monthstart = 1
type = "kvm"

[[hosts]]
name = "h2"
password = "p2"
host = "h2"
location = "us"
monthstart = 1
type = "kvm"

```

```bash
# 编译
yum install -y openssl-devel
cargo build --release

# 运行
./stat_srv
或
./stat_srv -c config.toml
或
RUST_BACKTRACE=1 RUST_LOG=trace ./stat_srv -c config.toml

## systemd
systemctl enable stat_srv
systemctl start stat_srv

## docker
docker-compose up -d
```

## 客户端
```bash
# 依赖安装
## Centos
sudo yum -y install epel-release
sudo yum -y install python3-pip gcc python3-devel
sudo python3 -m pip install psutil requests

## Ubuntu/Debian
sudo apt -y install python3-pip
sudo python3 -m pip install psutil requests

# 运行
python3 client-linux.py -a http://127.0.0.1:8080/report -u h1 -p p1

## systemd
systemctl enable stat_client
systemctl start stat_client
```

## TODO
```
1. manager api
```

### 管理接口
```json
[POST] http://127.0.0.1:8080/admin
{
	"cmd": "disable", // add, del, disable, enable
	"name": "h1",
    ...
}
```

## 参考
- https://github.com/cppla/ServerStatus

