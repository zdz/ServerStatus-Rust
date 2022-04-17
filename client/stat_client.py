#!/usr/bin/python3
# -*- coding: utf-8 -*-
# stat_client 跨平台 py 版本
# 依赖安装
# 先安装 python3 和 pip 和依赖
# 如 alpine linux : apk add py3-pip gcc python3-dev musl-dev linux-headers
# python3 -m pip install psutil requests
# 支持 Linux Windows macOS FreeBSD, OpenBSD, NetBSD Sun Solaris AIX

import os
import sys
import json
import time
import shlex
import errno
import timeit
import socket
import psutil
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


def get_cpu():
    # blocking
    return psutil.cpu_percent(interval=INTERVAL)


def get_sys_traffic():
    net_in, net_out = 0, 0
    net = psutil.net_io_counters(pernic=True)
    for k, v in net.items():
        if any([e in k for e in IFACE_IGNORE_LIST]):
            continue
        else:
            net_in += v[1]
            net_out += v[0]
    return net_in, net_out


def get_vnstat_traffic():
    now = datetime.now()
    vnstat_res = subprocess.check_output(
        "/usr/bin/vnstat --json m", shell=True)
    json_dict = json.loads(vnstat_res)
    network_in, network_out, m_network_in, m_network_out = (0, 0, 0, 0)
    for iface in json_dict.get("interfaces", []):
        name = iface["name"]

        if any([e in name for e in IFACE_IGNORE_LIST]):
            continue

        traffic = iface["traffic"]
        network_out += traffic["total"]["tx"]
        network_in += traffic["total"]["rx"]
        # print(name, json.dumps(iface["traffic"], indent=2))

        for month in traffic.get("month", []):
            if now.year != month["date"]["year"] or month["date"]["month"] != now.month:
                continue
            m_network_out += month["tx"]
            m_network_in += month["rx"]

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
    if(ip_version == 4):
        domain = "ipv4.google.com"
    elif(ip_version == 6):
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


def _net_speed():
    while True:
        avgrx, avgtx = 0, 0
        for name, stats in psutil.net_io_counters(pernic=True).items():
            if any([e in name for e in IFACE_IGNORE_LIST]):
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

    t_list.append(threading.Thread(
        target=_net_speed,
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


def sample(options, online4, online6):
    cpu_percent = get_cpu()
    uptime = get_uptime()
    load_1, load_5, load_15 = os.getloadavg(
    ) if 'linux' in sys.platform else (0.0, 0.0, 0.0)
    memory_total, memory_used, swap_total, swap_used = get_memory()
    hdd_total, hdd_used = get_hdd()

    stat_data = {}

    stat_data["frame"] = "data"
    stat_data['name'] = options.username
    stat_data['online4'] = online4
    stat_data['online6'] = online6

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

    stat_data['ping_10010'] = G_LOST_RATE.get('10010') * 100
    stat_data['ping_189'] = G_LOST_RATE.get('189') * 100
    stat_data['ping_10086'] = G_LOST_RATE.get('10086') * 100
    stat_data['time_10010'] = G_PING_TIME.get('10010')
    stat_data['time_189'] = G_PING_TIME.get('189')
    stat_data['time_10086'] = G_PING_TIME.get('10086')

    stat_data['vnstat'] = options.vnstat
    if options.vnstat:
        (network_in, network_out, m_network_in,
         m_network_out) = get_vnstat_traffic()
        stat_data['network_in'] = network_in
        stat_data['network_out'] = network_out
        stat_data['last_network_in'] = network_in - m_network_in
        stat_data['last_network_out'] = network_out - m_network_out
    else:
        net_in, net_out = get_sys_traffic()
        stat_data['network_in'] = net_in
        stat_data['network_out'] = net_out

    if options.disable_tupd:
        stat_data['tcp'], stat_data['udp'], stat_data['process'], stat_data['thread'] = 0, 0, 0, 0
    else:
        stat_data['tcp'], stat_data['udp'], stat_data['process'], stat_data['thread'] = tupd()
    return stat_data


def tcp_report(options):
    socket.setdefaulttimeout(5)
    start_rt_collect_t(options)

    online4 = get_network(4)
    online6 = get_network(6)

    arr = options.addr.replace("tcp://", "").split(":")
    server, port = arr
    auth_obj = {"frame": "auth", "user": options.username,
                "pass": options.password}
    while True:
        try:
            print("Connecting...")
            s = socket.create_connection((server, port))
            data = byte_str(s.recv(1024))
            if data.find("Authentication required") > -1:
                s.send(byte_str(json.dumps(auth_obj) + "\n"))
                data = byte_str(s.recv(1024))
                if data.find("Authentication successful") < 0:
                    print(data)
                    raise socket.error
            else:
                print(data)
                raise socket.error

            while True:
                stat_data = sample(options, online4, online6)
                print(json.dumps(stat_data))
                s.send(byte_str(json.dumps(stat_data) + "\n"))
        except KeyboardInterrupt:
            raise
        except socket.error:
            traceback.print_exc()
            print("Disconnected...")
            if 's' in locals().keys():
                del s
            time.sleep(3)
        except Exception as e:
            traceback.print_exc()
            print("Caught Exception:", e)
            if 's' in locals().keys():
                del s
            time.sleep(3)


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


def http_report(options):
    socket.setdefaulttimeout(5)
    start_rt_collect_t(options)

    online4 = get_network(4)
    online6 = get_network(6)

    if not any([online4, online6]):
        print("try get target network type {}".format(options.addr))
        ipv4, ipv6 = get_target_network(options.addr)
        online4 = ipv4
        online6 = ipv6

    auth = HTTPBasicAuth(options.username, options.password)
    sess = requests.Session()
    while True:
        try:
            stat_data = sample(options, online4, online6)
            print(json.dumps(stat_data))
            r = sess.post(options.addr, auth=auth, json=stat_data)
            print(r)
        except KeyboardInterrupt:
            raise
        except Exception as ex:
            traceback.print_exc()
            time.sleep(3)
            sess = requests.Session()


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
    parser.add_option("-p", "--pass", dest="password",
                      default="p1", help="auth pass [default: %default]")
    parser.add_option("-n", "--vnstat", default=False,
                      action="store_true", help="enable vnstat [default: %default]")
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

    (options, args) = parser.parse_args()
    print(json.dumps(options.__dict__, indent=2))

    if options.vnstat:
        if sys.platform.startswith("win"):
            raise RuntimeError("unsupport enable vnstat on win system")

    if options.addr.startswith("http"):
        http_report(options)
    elif options.addr.startswith("tcp"):
        tcp_report(options)
    else:
        print("invalid addr scheme")


if __name__ == '__main__':
    main()
