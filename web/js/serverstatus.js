(async () => {
    let stats = await (await fetch("/stats.json")).json()
    for (let i = 0; i < stats.servers.length; i++) {
        let node = document.createElement("div")
        node.id += "table-item-" + i
        node.className = "table-item"
        document.querySelector(".list-inner").append(node)
        if (stats.servers[i].online4 || stats.servers[i].online6) {
            getDetails(`#table-item-${i}`, stats.servers[i])
            document.querySelector(`#table-item-${i}`).innerHTML = `<div class="node">
    <img class="flag" src="https://z-fs.cols.ro/flags/square/${stats.servers[i].location.toLowerCase()}.svg" alt>
    <div>
        <div class="name">${stats.servers[i].alias}</div>
        <div class="location">${stats.servers[i].location}</div>
    </div>
</div>
<div class="type">${stats.servers[i].type}</div>
<div class="uptime">${stats.servers[i].uptime == "1 天" ? "1 Day" : stats.servers[i].uptime.replace(/天/, "Days")}</div>
<div class="network">${byteConvert(stats.servers[i].network_tx)}↑ ${byteConvert(stats.servers[i].network_rx)}↓</div>
<div class="traffic">${byteConvert(stats.servers[i].network_out)}↑ ${byteConvert(stats.servers[i].network_in)}↓</div>
<div class="cpu">
    <div class="progress">
        <div style="width: ${Math.round(stats.servers[i].cpu)}%; background-color: ${progressConvert(Math.round(stats.servers[i].cpu))};" class="progress-bar">
            <div>${Math.round(stats.servers[i].cpu)}%</div>
        </div>
    </div>
</div>
<div class="mem">
    <div class="progress">
        <div style="width: ${Math.round(stats.servers[i].memory_used / stats.servers[i].memory_total * 100)}%; background-color: ${progressConvert(Math.round(stats.servers[i].memory_used / stats.servers[i].memory_total * 100))}" class="progress-bar">
            <div>${Math.round(stats.servers[i].memory_used / stats.servers[i].memory_total * 100)}%</div>
        </div>
    </div>
</div>
<div class="hdd">
    <div class="progress">
        <div style="width: ${Math.round(stats.servers[i].hdd_used / stats.servers[i].hdd_total * 100)}%; background-color: ${progressConvert(Math.round(stats.servers[i].hdd_used / stats.servers[i].hdd_total * 100))}" class="progress-bar">
            <div>${Math.round(stats.servers[i].hdd_used / stats.servers[i].hdd_total * 100)}%</div>
        </div>
    </div>
</div>
<div class="status">
    <div class="status-dot" style="background-color: ${Math.round(stats.servers[i].cpu) <= 70 ? "" : "#faae42"};"></div>
    <div class="status-info">${Math.round(stats.servers[i].cpu) <= 70 ? "Available" : "Busy"}</div>
</div>`
            document.querySelector(`#table-item-${i}`).style.borderColor = Math.round(stats.servers[i].cpu) <= 70 ? "" : "#faae42"
        } else {
            document.querySelector(`#table-item-${i}`).innerHTML = `<div class="node">
    <img class="flag" src="https://z-fs.cols.ro/flags/square/${stats.servers[i].location.toLowerCase()}.svg" alt>
    <div>
        <div class="name">${stats.servers[i].alias}</div>
        <div class="location">${stats.servers[i].location}</div>
    </div>
</div>
<div class="type">${stats.servers[i].type}</div>
<div class="uptime">Offline</div>
<div class="network">-</div>
<div class="traffic">-</div>
<div class="cpu">
    <div class="progress">
        <div style="width: 100%; background-color: #e62965;" class="progress-bar">
            <div>Offline</div>
        </div>
    </div>
</div>
<div class="mem">
    <div class="progress">
        <div style="width: 100%; background-color: #e62965;" class="progress-bar">
            <div>Offline</div>
        </div>
    </div>
</div>
<div class="hdd">
    <div class="progress">
        <div style="width: 100%; background-color: #e62965;" class="progress-bar">
            <div>Offline</div>
        </div>
    </div>
</div>
<div class="status">
    <div class="status-dot" style="background-color: #a2a5b9;"></div>
    <div class="status-info">Offline</div>
</div>`
            document.querySelector(`#table-item-${i}`).style.borderColor = "#e62965"
        }
    }
})()

setInterval(() => {
    (async () => {
        let stats = await (await fetch("/stats.json")).json()
        for (let i = 0; i < stats.servers.length; i++) {
            try {
                if (stats.servers[i].online4 || stats.servers[i].online6) {
                    getDetails(`#table-item-${i}`, stats.servers[i])
                    document.querySelector(`#table-item-${i}`).style.borderColor = Math.round(stats.servers[i].cpu) <= 70 ? "" : "#faae42"
                    document.querySelector(`#table-item-${i} .flag`).src = `https://z-fs.cols.ro/flags/square/${stats.servers[i].location.toLowerCase()}.svg`
                    document.querySelector(`#table-item-${i} .location`).textContent = stats.servers[i].location
                    document.querySelector(`#table-item-${i} .type`).textContent = stats.servers[i].type
                    document.querySelector(`#table-item-${i} .uptime`).textContent = stats.servers[i].uptime == "1 天" ? "1 Day" : stats.servers[i].uptime.replace(/天/, "Days")
                    document.querySelector(`#table-item-${i} .network`).textContent = `${byteConvert(stats.servers[i].network_tx)}↑ ${byteConvert(stats.servers[i].network_rx)}↓`
                    document.querySelector(`#table-item-${i} .traffic`).textContent = `${byteConvert(stats.servers[i].network_out)}↑ ${byteConvert(stats.servers[i].network_in)}↓`
                    document.querySelector(`#table-item-${i} .cpu .progress-bar`).style.width = `${Math.round(stats.servers[i].cpu)}%`
                    document.querySelector(`#table-item-${i} .cpu .progress-bar`).style.backgroundColor = progressConvert(Math.round(stats.servers[i].cpu))
                    document.querySelector(`#table-item-${i} .cpu .progress-bar div`).textContent = `${Math.round(stats.servers[i].cpu)}%`
                    document.querySelector(`#table-item-${i} .mem .progress-bar`).style.width = `${Math.round(stats.servers[i].memory_used / stats.servers[i].memory_total * 100)}%`
                    document.querySelector(`#table-item-${i} .mem .progress-bar`).style.backgroundColor = progressConvert(Math.round(stats.servers[i].memory_used / stats.servers[i].memory_total * 100))
                    document.querySelector(`#table-item-${i} .mem .progress-bar div`).textContent = `${Math.round(stats.servers[i].memory_used / stats.servers[i].memory_total * 100)}%`
                    document.querySelector(`#table-item-${i} .hdd .progress-bar`).style.width = `${Math.round(stats.servers[i].hdd_used / stats.servers[i].hdd_total * 100)}%`
                    document.querySelector(`#table-item-${i} .hdd .progress-bar`).style.backgroundColor = progressConvert(Math.round(stats.servers[i].hdd_used / stats.servers[i].hdd_total * 100))
                    document.querySelector(`#table-item-${i} .hdd .progress-bar div`).textContent = `${Math.round(stats.servers[i].hdd_used / stats.servers[i].hdd_total * 100)}%`
                    document.querySelector(`#table-item-${i} .status-dot`).style.backgroundColor = Math.round(stats.servers[i].cpu) <= 70 ? "" : "#faae42"
                    document.querySelector(`#table-item-${i} .status-info`).textContent = Math.round(stats.servers[i].cpu) <= 70 ? "Available" : "Busy"
                } else {
                    document.querySelector(`#table-item-${i}`).onclick = null
                    document.querySelector(`#table-item-${i}`).style.borderColor = "#e62965"
                    document.querySelector(`#table-item-${i} .flag`).src = `https://z-fs.cols.ro/flags/square/${stats.servers[i].location.toLowerCase()}.svg`
                    document.querySelector(`#table-item-${i} .location`).textContent = stats.servers[i].location
                    document.querySelector(`#table-item-${i} .type`).textContent = stats.servers[i].type
                    document.querySelector(`#table-item-${i} .uptime`).textContent = "Offline"
                    document.querySelector(`#table-item-${i} .network`).textContent = "-"
                    document.querySelector(`#table-item-${i} .traffic`).textContent = "-"
                    document.querySelector(`#table-item-${i} .cpu .progress-bar`).style.width = "100%"
                    document.querySelector(`#table-item-${i} .cpu .progress-bar`).style.backgroundColor = "#e62965"
                    document.querySelector(`#table-item-${i} .cpu .progress-bar div`).textContent = "Offline"
                    document.querySelector(`#table-item-${i} .mem .progress-bar`).style.width = "100%"
                    document.querySelector(`#table-item-${i} .mem .progress-bar`).style.backgroundColor = "#e62965"
                    document.querySelector(`#table-item-${i} .mem .progress-bar div`).textContent = "Offline"
                    document.querySelector(`#table-item-${i} .hdd .progress-bar`).style.width = "100%"
                    document.querySelector(`#table-item-${i} .hdd .progress-bar`).style.backgroundColor = "#e62965"
                    document.querySelector(`#table-item-${i} .hdd .progress-bar div`).textContent = "Offline"
                    document.querySelector(`#table-item-${i} .status-dot`).style.backgroundColor = "#a2a5b9"
                    document.querySelector(`#table-item-${i} .status-info`).textContent = "Offline"
                }
            } catch {
                document.querySelector(`#table-item-${i}`).onclick = null
                document.querySelector(`#table-item-${i}`).style.borderColor = "#e62965"
                document.querySelector(`#table-item-${i} .uptime`).textContent = "Offline"
                document.querySelector(`#table-item-${i} .network`).textContent = "-"
                document.querySelector(`#table-item-${i} .traffic`).textContent = "-"
                document.querySelector(`#table-item-${i} .cpu .progress-bar`).style.width = "100%"
                document.querySelector(`#table-item-${i} .cpu .progress-bar`).style.backgroundColor = "#e62965"
                document.querySelector(`#table-item-${i} .cpu .progress-bar`).textContent = "Offline"
                document.querySelector(`#table-item-${i} .mem .progress-bar`).style.width = "100%"
                document.querySelector(`#table-item-${i} .mem .progress-bar`).style.backgroundColor = "#e62965"
                document.querySelector(`#table-item-${i} .mem .progress-bar`).textContent = "Offline"
                document.querySelector(`#table-item-${i} .hdd .progress-bar`).style.width = "100%"
                document.querySelector(`#table-item-${i} .hdd .progress-bar`).style.backgroundColor = "#e62965"
                document.querySelector(`#table-item-${i} .hdd .progress-bar`).textContent = "Offline"
                document.querySelector(`#table-item-${i} .status-dot`).style.backgroundColor = "#a2a5b9"
                document.querySelector(`#table-item-${i} .status-info`).textContent = "Offline"
            }
        }
    })()
}, 3000);

let getDetails = (item, data) => {
    document.querySelector(item).onclick = () => {
        Swal.fire({
            html: `<div style="margin-bottom: 20px; display: flex; align-items: center; justify-content: center;">
                <img style="margin-right: 10px; height: 50px;" src="https://z-fs.cols.ro/flags/rounded-rectangle/${data.location.toLowerCase()}.svg" alt>
                <h2>${data.name}</h2>
            </div>
            <div style="margin: 0 auto 10px; width: 350px; text-align: left; display: flex;"><p style="width: 35%;">Type:</p><p style="width: 65%;">${data.type}</p></div>
            <div style="margin: 0 auto 10px; width: 350px; text-align: left; display: flex;"><p style="width: 35%;">Uptime:</p><p style="width: 65%;">${data.uptime == "1 天" ? "1 Day" : data.uptime.replace(/天/, "Days")}</p></div>
            <div style="margin: 0 auto 10px; width: 350px; text-align: left; display: flex;"><p style="width: 35%;">CPU:</p><p style="width: 65%;">${data.cpu}%</p></div>
            <div style="margin: 0 auto 10px; width: 350px; text-align: left; display: flex;"><p style="width: 35%;">Memory:</p><p style="width: 65%;">${Math.round(data.memory_used / data.memory_total * 100)}% (${byteConvert2(data.memory_used)} / ${byteConvert2(data.memory_total)})</p></div>
            <div style="margin: 0 auto 10px; width: 350px; text-align: left; display: flex;"><p style="width: 35%;">Swap:</p><p style="width: 65%;">${data.swap_used == 0 ? "None" : `${Math.round(data.swap_used / data.swap_total * 100)}% (${byteConvert2(data.swap_used)} / ${byteConvert2(data.swap_total)})</p></div>`}</p></div>
            <div style="margin: 0 auto 10px; width: 350px; text-align: left; display: flex;"><p style="width: 35%;">HDD:</p><p style="width: 65%;">${Math.round(data.hdd_used / data.hdd_total * 100)}% (${byteConvert2(data.hdd_used * 1024)} / ${byteConvert2(data.hdd_total * 1024)})</p></div>
            <div style="margin: 0 auto 10px; width: 350px; text-align: left; display: flex;"><p style="width: 35%;">Network:</p><p style="width: 65%;">${byteConvert(data.network_tx)}↑ ${byteConvert(data.network_rx)}↓</p></div>
            <div style="margin: 0 auto 10px; width: 350px; text-align: left; display: flex;"><p style="width: 35%;">Traffic:</p><p style="width: 65%;">${byteConvert(data.network_out)}↑ ${byteConvert(data.network_in)}↓</p></div>`,
            showConfirmButton: false
        })
    }
}

let byteConvert = (data) => {
    if (data < 1024) {
        return data.toFixed(0) + 'B'
    } else if (data < 1024 * 1024) {
        return (data / 1024).toFixed(0) + 'K'
    } else if (data < 1024 * 1024 * 1024) {
        return (data / 1024 / 1024).toFixed(1) + 'M'
    } if (data < 1024 * 1024 * 1024 * 1024) {
        return (data / 1024 / 1024 / 1024).toFixed(2) + 'G'
    } else {
        return (data / 1024 / 1024 / 1024 / 1024).toFixed(2) + 'T'
    }
}

let byteConvert2 = (data) => {
    if (data < 1024) {
        return data.toFixed(0) + 'KiB'
    } else if (data < 1024 * 1024) {
        return (data / 1024).toFixed(0) + 'MiB'
    } else if (data < 1024 * 1024 * 1024) {
        return (data / 1024 / 1024).toFixed(1) + 'GiB'
    } else {
        return (data / 1024 / 1024 / 1024).toFixed(2) + 'TiB'
    }
}

let progressConvert = (data) => {
    if (data <= 70) {
        return ""
    } else if (data <= 90) {
        return "#faae42"
    } else {
        return "#e62965"
    }
}

