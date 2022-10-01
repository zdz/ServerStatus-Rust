'''
因为esxi 默认就是20s刷新一次,所以暂时就只能写成这个样子了🤷‍♂️
python3 -m pip install pyVmomi requests schedule
'''
from pyVim.connect import SmartConnectNoSSL, Disconnect
from pyVmomi import vim
from datetime import timedelta
import atexit
from optparse import OptionParser
from requests.auth import HTTPBasicAuth
import requests
import json
import traceback
import schedule
import copy

esxi_host = None

class EsxiHost:
    _si = None
    _username: str = None
    _password: str = None
    _vc_ip: str = None
    _vc_port: str = None
    _perf_dict: dict = {}

    def __init__(self, username: str, password: str, vc_ip: str,
                 vc_port: str) -> None:
        self._username = username
        self._password = password
        self._vc_ip = vc_ip
        self._vc_port = vc_port

    def _connect_to_server(self):
        try:
            self._si = SmartConnectNoSSL(host=self._vc_ip,
                                         user=self._username,
                                         pwd=self._password,
                                         port=self._vc_port)
            atexit.register(Disconnect, self._si)
            return self._si
        except Exception as e:
            print(e)
            self._si = None

    def _si_check_wrapper(func):

        def wrapper(self, *args, **kwargs):
            if self._si is None:
                self._connect_to_server()
            return func(self, *args, **kwargs)

        return wrapper

    @_si_check_wrapper
    def get_content(self):
        return self._si.RetrieveContent()

    @_si_check_wrapper
    def get_esxi_time(self):
        return self._si.CurrentTime()

    @_si_check_wrapper
    def get_retrieve_content(self):
        return self._si.RetrieveContent()

    @_si_check_wrapper
    def get_perf_dict(self) -> dict:
        self._perf_dict.clear()
        content = self.get_retrieve_content()
        perf_list = content.perfManager.perfCounter
        for counter in perf_list:
            counter_full = "{}.{}.{}".format(counter.groupInfo.key,
                                             counter.nameInfo.key,
                                             counter.rollupType)
            self._perf_dict[counter_full] = counter.key
        return self._perf_dict

    @_si_check_wrapper
    def get_esxi_host_obj(self):
        content = self.get_retrieve_content()
        return content.viewManager.CreateContainerView(content.rootFolder,
                                                       [vim.HostSystem], True)

    def is_alive(self) -> bool:
        if self._si is None:
            return False
        try:
            self.get_esxi_time()
        except Exception as e:
            self._si = None
            return False
        return True


class EsxiHostUtils:
    # https://communities.vmware.com/t5/Storage-Performance/vCenter-Performance-Counters/ta-p/2790328
    statistics_interval_time: int = 20  # 因为esxi默认就是20s刷新一次，所以这个间隔是一定不会报错的

    def __init__(self, content, vc_time, perf_dict, host_obj) -> None:
        self.content = content
        self.vc_time = vc_time
        self.perf_dict = perf_dict
        self.host = host_obj

    def get_type_id(self, query_type):
        counter_id = self.perf_dict[query_type]
        return counter_id

    def get_network_tx(self):
        stat_network_tx = self.build_query(
            query_type="net.transmitted.average")
        network_tx =  int(round(
            sum(stat_network_tx[0].value[0].value) /
            self.statistics_interval_time,2)) * 1024
        return network_tx * 1024

    def get_network_rx(self):
        stat_network_rx = self.build_query(query_type="net.received.average")
        network_rx = int(round(
            sum(stat_network_rx[0].value[0].value) /
            self.statistics_interval_time,2)) * 1024

        return network_rx * 1024

    def get_cpu_usage(self):
        state_cpu_usage = self.build_query(query_type="cpu.usage.average")
        cpu_usage = float(
            round((((sum(state_cpu_usage[0].value[0].value))) / 100 /
                   self.statistics_interval_time), 1))
        return cpu_usage

    def get_disk_capacity_and_usage(self, content) -> tuple:
        capacity = 0
        free_size = 0
        for datacenter in content.rootFolder.childEntity:
            datastores = datacenter.datastore
            for ds in datastores:
                capacity += ds.summary.capacity
                free_size += ds.summary.freeSpace

        capacity = int(round(capacity / 1024.0 / 1024.0, 0))
        freesize = int(round(free_size / 1024.0 / 1024.0, 0))
        return (capacity, freesize)

    def build_query(self, query_type: str):
        counter_id = self.get_type_id(query_type)
        metric_id = vim.PerformanceManager.MetricId(counterId=counter_id,
                                                    instance="")
        start_time = self.vc_time - timedelta(
            seconds=(self.statistics_interval_time))
        # end_time = vc_time - timedelta(seconds=0)
        end_time = self.vc_time
        query = vim.PerformanceManager.QuerySpec(intervalId=20,
                                                 entity=self.host,
                                                 metricId=[metric_id],
                                                 startTime=start_time,
                                                 endTime=end_time)
        perf_results = self.content.perfManager.QueryPerf(querySpec=[query])
        if perf_results:
            return perf_results
        else:
            pass


PostData = {
    "frame": "data",
    "version": "py",
    "gid": '',
    "alias": '',
    "name": '',
    "weight": 0,
    "vnstat": False,
    "notify": False,
    "online4": True,
    "oneline6": '',
    "uptime": 0,
    "load_1": 0,
    "load_5": 0,
    "load_15": 0,
    "memory_total": 0,
    "memory_used": 0,
    "swap_total": 0,
    "swap_used": 0,
    "hdd_total": 0,
    "hdd_used": 0,
    "cpu": 0,
    "network_rx": 0,
    "network_tx": 0,
    "network_in": 0,
    "network_out": 0,
    "ping_10010": 0,
    "ping_189": 0,
    "ping_10086": 0,
    "time_10010": 0,
    "time_189": 0,
    "time_10086": 0,
    "tcp": 0,
    "udp": 0,
    "process": 0,
    "thread": 0,
}
# class PostDataInfo:
#     frame = "data"
#     version = "py"
#     gid = ""
#     alias = ""
#     name: str = None
#     weight: int = 0
#     vnstat: bool = False
#     notify: bool = True
#     online4: bool = True
#     oneline6: bool = False  # todo more elegant
#     uptime: int = 0
#     load_1 = 0
#     load_5 = 0
#     load_15 = 0
#     memory_total: str = None
#     memory_used: str = None
#     swap_total: str = "0"
#     swap_used: str = "0"
#     hdd_total: str = None
#     hdd_used: str = None
#     cpu: float = None
#     network_rx: int = 0
#     network_tx: int = 0
#     network_in: int = 22257511524
#     network_out: int = 52842748137
#     ping_10010: int = 0.0
#     ping_189: int = 0.0
#     ping_10086: int = 0.0
#     time_10010: int = 0
#     time_189: int = 0
#     time_10086: int = 0
#     tcp = 0
#     udp = 0
#     process = 0
#     thread = 0

#     def __init__(self):

#         pass

#     def keys(self):

#         # return [
#         #     ele for ele in self.__dir__() if "_" not in ele and ele != "keys"
#         # ]
#         return ("frame", "version", "gid", "alias", "name", "weight", "vnstat",
#                 "notify", "online4", "oneline6", "uptime", "load_1", "load_5",
#                 "load_15", "memory_total", "memory_used", "swap_total",
#                 "swap_used", "hdd_total", "hdd_used", "cpu", "network_rx",
#                 "network_tx", "network_in","network_out","ping_10010", "ping_189", "ping_10086",
#                 "time_10010", "time_189", "time_10086", "tcp", "udp",
#                 "process", "thread")

#     def __getitem__(self, item):
#         return getattr(self, item)


def gather_data(username: str, password: str, vc_ip: str, vc_port: str,
                host_username: str) -> dict:

    global esxi_host
    if esxi_host is None:
        esxi_host = EsxiHost(username, password, vc_ip, vc_port)

    post_data = copy.deepcopy(PostData)
    host_view = esxi_host.get_esxi_host_obj()
    if len(host_view.view) > 1:
        raise "暂时不支持集群或vcenter"
    # for host in host_view.view:
    host = host_view.view[0]
    stats = host.summary.quickStats
    hardware = host.hardware
    post_data["name"] = host_username
    post_data["uptime"] = stats.uptime
    post_data["memory_total"] = int(round(hardware.memorySize / 1024, 0))
    post_data["memory_used"] = stats.overallMemoryUsage * 1024
    esxi_utils = EsxiHostUtils(content=esxi_host.get_content(),
                               vc_time=esxi_host.get_esxi_time(),
                               perf_dict=esxi_host.get_perf_dict(),
                               host_obj=host)
    volume_capacity, volume_free_sapce = esxi_utils.get_disk_capacity_and_usage(
        content=esxi_host.get_content())
    volume_used_space = volume_capacity - volume_free_sapce
    post_data["hdd_total"] = volume_capacity
    post_data["hdd_used"] = volume_used_space
    post_data["cpu"] = esxi_utils.get_cpu_usage()
    post_data["network_rx"] = esxi_utils.get_network_rx()
    post_data["network_tx"] = esxi_utils.get_network_tx()
    return post_data


def report(username: str, password: str, addr: str, data):
    try:
        sess = requests.Session()
        ssr_auth = "single"
        http_headers = {"ssr-auth": ssr_auth}
        auth = HTTPBasicAuth(username, password)
        r = sess.post(addr, auth=auth, json=data, headers=http_headers)
        print(r)
    except Exception as e:
        print(e)
        traceback.print_exc()


def run(options):
    s_addr = options.addr
    s_username = options.username
    s_gid = options.gid
    s_alias = options.alias
    s_password = options.password
    esxi_username = options.esxi_username
    esxi_password = options.esxi_password
    esxi_addr = options.esxi_addr
    esxi_port = options.esxi_port
    data = gather_data(esxi_username, esxi_password, esxi_addr, esxi_port,
                       s_username)
    report(s_username, s_password, s_addr, data)


if __name__ == "__main__":

    usage = """usage: python3 %prog [options] arg
    eg:
        python3 %prog -a http://127.0.0.1:8080/report -u h1 -p p1 --esxiuser root --esxipasswd password --esxiaddr 192.169.1.2 --esxiport 443
    """
    parser = OptionParser(usage)

    parser.add_option("-a",
                      "--addr",
                      dest="addr",
                      default="http://10.10.10.100/report",
                      help="http/tcp addr [default: %default]")
    parser.add_option("-u",
                      "--user",
                      dest="username",
                      default="h1",
                      help="auth user [default: %default]")
    parser.add_option("-g",
                      "--gid",
                      dest="gid",
                      default="",
                      help="group id [default: %default]")
    parser.add_option("--alias",
                      dest="alias",
                      default="unknown",
                      help="alias for host [default: %default]")
    parser.add_option("-p",
                      "--pass",
                      dest="password",
                      default="password",
                      help="auth pass [default: %default]")
    parser.add_option("-e",
                      "--esxiuser",
                      dest="esxi_username",
                      default="root",
                      help="esxi username [default: %default]")
    parser.add_option("-f",
                      "--esxipasswd",
                      dest="esxi_password",
                      default="password",
                      help="esxi password [default: %default]")
    parser.add_option("-w",
                      "--esxiaddr",
                      dest="esxi_addr",
                      default="192.168.1.2",
                      help="esxi addr [default: %default]")
    parser.add_option("-r",
                      "--esxiport",
                      dest="esxi_port",
                      default="443",
                      help="esxi port [default: %default]")
    (options, args) = parser.parse_args()
    print(json.dumps(options.__dict__, indent=2))
    schedule.every(5).seconds.do(run, options)
    print("running... report every 5 seconds")
    while True:
        try:
            
            schedule.run_pending()
            # run(options)
        except KeyboardInterrupt:
            exit(0)