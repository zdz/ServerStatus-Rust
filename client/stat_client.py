#!/usr/bin/python3
# -*- coding: utf-8 -*-
# stat_client 跨平台 py 版本
# 依赖安装
# 先安装 python3 和 pip 和依赖
# 如 alpine linux : apk add py3-pip gcc python3-dev musl-dev linux-headers
# python3 -m pip install psutil requests py-cpuinfo
# 支持 Linux Windows macOS FreeBSD, OpenBSD, NetBSD Sun Solaris AIX

import os
import sys
import copy
import json
import time
import shlex
import errno
import timeit
import socket
import psutil
import hashlib
import threading
import subprocess
import requests
import traceback
from queue import Queue
from datetime import datetime
from requests.auth import HTTPBasicAuth
from optparse import OptionParser

CU = "cu.tz.cloudcpp.com:80"
CT = "ct.tz.cloudcpp.com:80"
CM = "cm.tz.cloudcpp.com:80"
IFACE_IGNORE_LIST = ["lo", "tun", "docker",
                     "vnet", "veth", "vmbr", "kube", "br-"]

PROBE_PROTOCOL_PREFER = 'ipv4'
PING_PACKET_HISTORY_LEN = 100
INTERVAL = 1
G_IP_INFO = None
G_SYS_INFO = None


def get_uptime():
    # uptime = datetime.now() - datetime.fromtimestamp(psutil.boot_time())
    # print(uptime)
    return int(time.time() - psutil.boot_time())


def get_memory():
    vm = psutil.virtual_memory()
    sm = psutil.swap_memory()
    return int(vm.total / 1024.0), int(vm.used / 1024.0), int(sm.total / 1024.0), int(sm.used / 1024.0)


def get_hdd():
    valid_fs = set(["ext4", "ext3", "ext2", "reiserfs", "jfs", "btrfs",
                   "fuseblk", "zfs", "simfs", "ntfs", "fat32", "exfat", "xfs"])
    disks, size, used = dict(), 0, 0
    for disk in psutil.disk_partitions():
        if disk.device not in disks and disk.fstype.lower() in valid_fs:
            disks[disk.device] = disk.mountpoint
    for disk in disks.values():
        usage = psutil.disk_usage(disk)
        size += usage.total
        used += usage.used
    return int(size / 1024.0 / 1024.0), int(used / 1024.0 / 1024.0)


def get_cpu(options):
    # blocking
    # return psutil.cpu_percent(interval=INTERVAL)
    return psutil.cpu_percent(interval=int(options.interval))


def get_sys_traffic(options):
    net_in, net_out = 0, 0
    net = psutil.net_io_counters(pernic=True)
    for k, v in net.items():
        if skip_iface(k, options):
            continue
        else:
            net_in += v[1]
            net_out += v[0]
    return net_in, net_out


def get_vnstat_traffic(options):
    now = datetime.now()
    vnstat_res = subprocess.check_output(
        "/usr/bin/vnstat --json m", shell=True)
    json_dict = json.loads(vnstat_res)
    network_in, network_out, m_network_in, m_network_out = (0, 0, 0, 0)
    json_version = json_dict.get("jsonversion", "2")
    for iface in json_dict.get("interfaces", []):
        name, bandwidth_factor, mouth_field = "invalid", 1, "month"
        if json_version == "1":
            name = iface["id"]
            bandwidth_factor = 1024
            mouth_field = "months"
        else:
            name = iface["name"]

        if skip_iface(name, options):
            continue

        traffic = iface["traffic"]
        network_out += traffic["total"]["tx"] * bandwidth_factor
        network_in += traffic["total"]["rx"] * bandwidth_factor
        # print(name, json.dumps(iface["traffic"], indent=2))

        for month in traffic.get(mouth_field, []):
            if now.year != month["date"]["year"] or month["date"]["month"] != now.month:
                continue
            m_network_out += month["tx"] * bandwidth_factor
            m_network_in += month["rx"] * bandwidth_factor

    return (network_in, network_out, m_network_in, m_network_out)


def tupd():
    """tcp/udp/process/thread count"""
    t, u, p, d = 0, 0, 0, 0
    tcp_set, udp_set = set(), set()
    try:
        t = len(psutil.net_connections("tcp"))
        u = len(psutil.net_connections("udp"))
        for proc in psutil.process_iter():
            p += 1
            try:
                d += proc.num_threads()
            except:
                pass
        return t, u, p, d
    except Exception as ex:
        traceback.print_exc()
    return 0, 0, 0, 0


def get_network(ip_version):
    if (ip_version == 4):
        domain = "ipv4.google.com"
    elif (ip_version == 6):
        domain = "ipv6.google.com"
    try:
        socket.create_connection((domain, 80), 2).close()
        return True
    except:
        return False


G_LOST_RATE = {
    '10010': 0.0,
    '189': 0.0,
    '10086': 0.0
}
G_PING_TIME = {
    '10010': 0,
    '189': 0,
    '10086': 0
}
G_NET_SPEED = {
    'netrx': 0.0,
    'nettx': 0.0,
    'clock': 0.0,
    'diff': 0.0,
    'avgrx': 0,
    'avgtx': 0
}


def _ping_thread(target, mark):
    packet_lost = 0
    packet_queue = Queue(maxsize=PING_PACKET_HISTORY_LEN)

    host, port = target.split(":")

    ip = host
    if host.count(':') < 1:     # if not plain ipv6 address, means ipv4 address or hostname
        try:
            if PROBE_PROTOCOL_PREFER == 'ipv4':
                ip = socket.getaddrinfo(host, None, socket.AF_INET)[0][4][0]
            else:
                ip = socket.getaddrinfo(host, None, socket.AF_INET6)[0][4][0]
        except Exception as ex:
            traceback.print_exc()

    while True:
        if packet_queue.full():
            if packet_queue.get() == 0:
                packet_lost -= 1
        try:
            b = timeit.default_timer()
            socket.create_connection((ip, port), timeout=1).close()
            G_PING_TIME[mark] = int((timeit.default_timer() - b) * 1000)
            packet_queue.put(1)
        except socket.error as error:
            if error.errno == errno.ECONNREFUSED:
                G_PING_TIME[mark] = int((timeit.default_timer() - b) * 1000)
                packet_queue.put(1)
            # elif error.errno == errno.ETIMEDOUT:
            else:
                packet_lost += 1
                packet_queue.put(0)

        if packet_queue.qsize() > 30:
            G_LOST_RATE[mark] = float(packet_lost) / packet_queue.qsize()

        time.sleep(INTERVAL)


def _net_speed(options):
    while True:
        avgrx, avgtx = 0, 0
        for name, stats in psutil.net_io_counters(pernic=True).items():
            if skip_iface(name, options):
                continue
            avgrx += stats.bytes_recv
            avgtx += stats.bytes_sent
        now_clock = time.time()
        G_NET_SPEED["diff"] = now_clock - G_NET_SPEED["clock"]
        G_NET_SPEED["clock"] = now_clock
        G_NET_SPEED["netrx"] = int(
            (avgrx - G_NET_SPEED["avgrx"]) / G_NET_SPEED["diff"])
        G_NET_SPEED["nettx"] = int(
            (avgtx - G_NET_SPEED["avgtx"]) / G_NET_SPEED["diff"])
        G_NET_SPEED["avgrx"] = avgrx
        G_NET_SPEED["avgtx"] = avgtx
        time.sleep(INTERVAL)


def start_rt_collect_t(options):
    """realtime data collect"""
    t_list = []
    if not options.disable_ping:
        t_list.append(threading.Thread(
            target=_ping_thread,
            kwargs={
                'target': options.cu,
                'mark': '10010',
            }
        ))
        t_list.append(threading.Thread(
            target=_ping_thread,
            kwargs={
                'target': options.ct,
                'mark': '189',
            }
        ))
        t_list.append(threading.Thread(
            target=_ping_thread,
            kwargs={
                'target': options.cm,
                'mark': '10086',
            }
        ))

    # ip info
    if not options.disable_extra:
        t_list.append(threading.Thread(
            target=refresh_ip_info,
        ))

    # net speed
    t_list.append(threading.Thread(
        target=_net_speed,
        kwargs={
            'options': options,
        }
    ))

    for t in t_list:
        t.setDaemon(True)
        t.start()


def byte_str(object):
    """bytes to str, str to bytes"""
    if isinstance(object, str):
        return object.encode(encoding="utf-8")
    elif isinstance(object, bytes):
        return bytes.decode(object)
    else:
        print(type(object))


def sample(options, stat_base):
    cpu_percent = int(get_cpu(options))
    uptime = get_uptime()
    load_1, load_5, load_15 = os.getloadavg(
    ) if 'linux' in sys.platform else (0.0, 0.0, 0.0)
    memory_total, memory_used, swap_total, swap_used = get_memory()
    hdd_total, hdd_used = get_hdd()

    stat_data = copy.deepcopy(stat_base)

    stat_data['uptime'] = uptime

    stat_data['load_1'] = load_1
    stat_data['load_5'] = load_5
    stat_data['load_15'] = load_15

    stat_data['memory_total'] = memory_total
    stat_data['memory_used'] = memory_used
    stat_data['swap_total'] = swap_total
    stat_data['swap_used'] = swap_used
    stat_data['hdd_total'] = hdd_total
    stat_data['hdd_used'] = hdd_used
    stat_data['cpu'] = cpu_percent

    stat_data['network_rx'] = G_NET_SPEED.get("netrx")
    stat_data['network_tx'] = G_NET_SPEED.get("nettx")

    stat_data['ping_10010'] = int(G_LOST_RATE.get('10010') * 100)
    stat_data['ping_189'] = int(G_LOST_RATE.get('189') * 100)
    stat_data['ping_10086'] = int(G_LOST_RATE.get('10086') * 100)
    stat_data['time_10010'] = G_PING_TIME.get('10010')
    stat_data['time_189'] = G_PING_TIME.get('189')
    stat_data['time_10086'] = G_PING_TIME.get('10086')

    if options.vnstat:
        (network_in, network_out, m_network_in,
         m_network_out) = get_vnstat_traffic(options)
        stat_data['network_in'] = network_in
        stat_data['network_out'] = network_out
        stat_data['last_network_in'] = network_in - m_network_in
        stat_data['last_network_out'] = network_out - m_network_out
    else:
        net_in, net_out = get_sys_traffic(options)
        stat_data['network_in'] = net_in
        stat_data['network_out'] = net_out

    if options.disable_tupd:
        stat_data['tcp'], stat_data['udp'], stat_data['process'], stat_data['thread'] = 0, 0, 0, 0
    else:
        stat_data['tcp'], stat_data['udp'], stat_data['process'], stat_data['thread'] = tupd()

    if not options.disable_extra:
        if G_IP_INFO:
            stat_data['ip_info'] = G_IP_INFO
        if G_SYS_INFO:
            stat_data['sys_info'] = G_SYS_INFO

    return stat_data


def get_target_network(url):
    ipv4, ipv6 = False, False
    arr = url.split("/")
    proto = arr[0].replace(":", "")
    t = arr[2].split(":")
    if len(t) == 2:
        host, port = t
    else:
        host = t[0]
        port = 443 if proto == "https" else 80
    for response in socket.getaddrinfo(host, port):
        family, _, _, _, sockaddr = response
        print(family, sockaddr)
        if family == socket.AddressFamily.AF_INET:
            ipv4 = True
        elif family == socket.AddressFamily.AF_INET6:
            ipv6 = True
    return ipv4, ipv6


def http_report(options, stat_base):
    socket.setdefaulttimeout(5)
    start_rt_collect_t(options)

    online4 = get_network(4)
    online6 = get_network(6)
    if not any([online4, online6]):
        print("try get target network type {}".format(options.addr))
        ipv4, ipv6 = get_target_network(options.addr)
        online4 = ipv4
        online6 = ipv6

    stat_base['online4'] = online4
    stat_base['online6'] = online6

    ssr_auth = "single"
    auth_user = options.username
    if len(options.gid) > 0:
        ssr_auth = "group"
        auth_user = options.gid

    http_headers = {"ssr-auth": ssr_auth}
    auth = HTTPBasicAuth(auth_user, options.password)
    print(http_headers, auth)
    sess = requests.Session()
    while True:
        try:
            stat_data = sample(options, stat_base)
            print(json.dumps(stat_data))
            r = sess.post(options.addr, auth=auth,
                          json=stat_data, headers=http_headers)
            print(r)
        except KeyboardInterrupt:
            raise
        except Exception as ex:
            traceback.print_exc()
            time.sleep(3)
            sess = requests.Session()


IP_API_URL = "http://ip-api.com/json?fields=status,message,continent,continentCode,country,countryCode,region,regionName,city,district,zip,lat,lon,timezone,isp,org,as,asname,query&lang=zh-CN"


def refresh_ip_info():
    """ip info"""
    while True:
        try:
            r = requests.get(IP_API_URL, timeout=5)
            resp = r.json()
            # print(json.dumps(resp, indent=2))
            ip_info = {
                "query": resp.get("query", "unknown"),
                "source": "ip-api.com",
                "continent": resp.get("continent", "unknown"),
                "country": resp.get("country", "unknown"),
                "region_name": resp.get("regionName", "unknown"),
                "city": resp.get("city", "unknown"),
                "isp": resp.get("isp", "unknown"),
                "org": resp.get("org", "unknown"),
                "as": resp.get("as", "unknown"),
                "asname": resp.get("asname", "unknown"),
                "lat": resp.get("lat", 0),
                "lon": resp.get("lon", 0),
                "timezone": resp.get("timezone", "Asia/Shanghai"),
            }
            # print(json.dumps(ip_info, indent=2))
            global G_IP_INFO
            G_IP_INFO = ip_info
        except Exception as ex:
            traceback.print_exc()
        # /1h
        time.sleep(3600)


def get_sys_info(options):
    """sys info"""
    # pip3 install py-cpuinfo
    import platform
    from cpuinfo import get_cpu_info
    cpu_info = get_cpu_info()

    sys_info = {
        "name": options.username,
        "version": "stat_client.py",
        "os_name": platform.system(),
        "os_arch": platform.machine(),
        "os_family": "unknown",
        "os_release": platform.platform(),
        "kernel_version": platform.release(),
        "cpu_num": cpu_info.get("count", 0),
        "cpu_brand": cpu_info.get("brand_raw", "unknown"),
        "cpu_vender_id": cpu_info.get("vendor_id_raw", "unknown"),
        "host_name": platform.node(),
    }

    return sys_info


def gen_sys_id(sys_info):
    """"""
    SYS_ID_FILE = ".server_status_sys_id"
    if os.path.exists(SYS_ID_FILE):
        with open(SYS_ID_FILE, 'r') as f:
            sys_id = f.read()
            print(f"read sys_id from {SYS_ID_FILE}")
            return sys_id.strip()

    s = "{}/{}/{}/{}/{}/{}/{}/{}".format(
        sys_info.get("host_name", "unknown"),
        sys_info.get("os_name", "unknown"),
        sys_info.get("os_arch", "unknown"),
        sys_info.get("os_family", "unknown"),
        sys_info.get("os_release", "unknown"),
        sys_info.get("kernel_version", "unknown"),
        sys_info.get("cpu_brand", "unknown"),
        psutil.boot_time(),
    )
    sys_id = hashlib.md5(s.encode("utf-8")).hexdigest()
    with open(SYS_ID_FILE, "w") as wf:
        wf.write(sys_id)
        print(f"save sys_id to {SYS_ID_FILE} succ")
    return sys_id


def skip_iface(name, options):
    if len(options.iface) > 0:
        if any([name == e for e in options.iface]):
            return False
        return True
    if any([e in name for e in options.exclude_iface]):
        return True
    return False


def main():
    usage = """usage: python3 %prog [options] arg
    eg:
        python3 %prog -a http://127.0.0.1:8080/report -u h1 -p p1
        python3 %prog -a http://127.0.0.1:8080/report -u h1 -p p1 -n
    """
    parser = OptionParser(usage)

    parser.add_option("-a", "--addr", dest="addr", default="http://127.0.0.1:8080/report",
                      help="http/tcp addr [default: %default]")
    parser.add_option("-u", "--user", dest="username",
                      default="h1", help="auth user [default: %default]")
    parser.add_option("-g", "--gid", dest="gid",
                      default="", help="group id [default: %default]")
    parser.add_option("--alias", dest="alias",
                      default="unknown", help="alias for host [default: %default]")
    parser.add_option("-p", "--pass", dest="password",
                      default="p1", help="auth pass [default: %default]")
    parser.add_option("-n", "--vnstat", default=False,
                      action="store_true", help="enable vnstat [default: %default]")
    parser.add_option("--disable-extra", default=False,
                      action="store_true", help="disable extra info report [default: %default]")
    parser.add_option("--disable-ping", default=False,
                      action="store_true", help="disable ping [default: %default]")
    parser.add_option("--disable-tupd", default=False,
                      action="store_true", help="disable t/u/p/d [default: %default]")
    parser.add_option("--cm", dest="cm", default=CM,
                      help="China Mobile probe addr [default: %default]")
    parser.add_option("--ct", dest="ct", default=CT,
                      help="China Telecom probe addr [default: %default]")
    parser.add_option("--cu", dest="cu", default=CU,
                      help="China Unicom probe addr [default: %default]")
    parser.add_option("-w", "--weight", dest="weight",
                      default=0, help="weight for rank [default: %default]")
    parser.add_option("--disable-notify", default=False,
                      action="store_true", help="disable notify [default: %default]")
    parser.add_option("-t", "--type", dest="type",
                      default="", help="host type [default: %default]")
    parser.add_option("--location", dest="location",
                      default="", help="location [default: %default]")
    parser.add_option("-i", "--iface", dest="iface",
                      default="", help="iface list, eg: eth0,eth1 [default: %default]")
    parser.add_option("-e", "--exclude-iface", dest="exclude_iface",
                      default="lo,docker,vnet,veth,vmbr,kube,br-",
                      help="exclude iface [default: %default]")
    parser.add_option("--interval", dest="interval",
                      default=1, help="report interval [default: %default]")

    (options, args) = parser.parse_args()

    def parse_iface_list(ifaces): return list(
        filter(lambda s: len(s), map(str.strip, ifaces.split(","))))
    options.iface = parse_iface_list(options.iface)
    options.exclude_iface = parse_iface_list(options.exclude_iface)
    print(json.dumps(options.__dict__, indent=2))

    if options.vnstat:
        if sys.platform.startswith("win"):
            raise RuntimeError("unsupported: enable vnstat on win os")

    # sys info
    sys_info = get_sys_info(options)
    sys_id = gen_sys_id(sys_info)
    print("sys info: {}".format(json.dumps(sys_info, indent=2)))
    print("sys id: {}".format(sys_id))
    if not options.disable_extra:
        global G_SYS_INFO
        G_SYS_INFO = sys_info

    stat_base = {}
    stat_base["frame"] = "data"
    stat_base['version'] = "py"
    stat_base['gid'] = ""
    stat_base['alias'] = ""
    stat_base['name'] = options.username
    stat_base['weight'] = options.weight
    stat_base['vnstat'] = options.vnstat
    stat_base['notify'] = True

    if len(options.gid) > 0:
        stat_base["gid"] = options.gid
        if options.username == "h1":
            stat_base['name'] = sys_id
        if options.alias == "unknown":
            stat_base['alias'] = stat_base['name']
        else:
            stat_base['alias'] = options.alias

    if options.disable_notify:
        stat_base['notify'] = False
    if len(options.type) > 0:
        stat_base['type'] = options.type
    if len(options.location) > 0:
        stat_base['location'] = options.location

    print("stat_base: {}".format(json.dumps(stat_base, indent=2)))
    # sys.exit(0)

    if options.addr.startswith("http"):
        http_report(options, stat_base)
    elif options.addr.startswith("grpc"):
        raise RuntimeError("grpc unsupported")
    else:
        print("invalid addr scheme")


if __name__ == '__main__':
    main()
