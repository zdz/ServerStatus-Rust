# 如何在 Heroku 上部署 Rust 版 ServerStatus 云探针

本教程详细介绍如何在 Heroku 平台上部署 Rust 版的 ServerStatus 云探针。

## 一、安装 Heroku CLI

Heroku CLI 是管理和运行 Heroku 应用程序必备的命令行工具。

### 1. 在 Ubuntu/Debian 上安装

安装 CLI , 下载和编辑文件需要以下工具:

unzip, git, curl, wget, nano, vim


使用官方安装脚本一键安装:

```bash  
curl https://cli-assets.heroku.com/install.sh | sh
```

安装多账户管理插件:

```bash
heroku plugins:install heroku-accounts 
```

### 2. 登录账户 

以 admin@heroku.com 为例,账号密码为 Heroku Dashboard 上的 API Key（Acount 里）:

![api](https://github.com/kissyouhunter/ServerStatus-Rust/raw/heroku/heroku/readme-pics/api.jpg)

```bash
heroku accounts:add admin@heroku.com  
Enter your Heroku credentials.
Email: admin@heroku.com  
Password: ********
```

切换账户命令:

```bash
heroku accounts:set 账户别名  
```

## 二、部署应用

### 1. 准备文件

在本地创建文件夹,下载所需文件:

- stat_server:编译好的 Rust 二进制程序 
- config.toml:配置文件
- Procfile:用于定义 dyno 运行命令

```bash
mkdir ServerStatus-Rust && cd ServerStatus-Rust

# 下载示例以 v1.7.2 为例 
wget https://github.com/zdz/ServerStatus-Rust/releases/download/v1.7.2/server-x86_64-unknown-linux-musl.zip  

unzip server-x86_64-unknown-linux-musl.zip
rm -f stat_server.service server-x86_64-unknown-linux-musl.zip

wget https://github.com/kissyouhunter/ServerStatus-Rust/raw/heroku/heroku/Procfile
```

### 2. 创建应用

在 Heroku CLI 中创建应用,以 `serverstatus-rust-heroku` 为例:

```
heroku apps:create serverstatus-rust-heroku
```

设置自定义 buildpacks:

```
heroku buildpacks:set https://github.com/kissyouhunter/heroku-empty-build.git -a serverstatus-rust-heroku
```

### 3. 部署代码

初始化 git 仓库并将 heroku app 作为 remote:

```
git init
git config user.email "admin@heroku.com"
heroku git:remote -a serverstatus-rust-heroku 
git checkout -b main
git branch -D master
```

添加文件到 git 中后 push 到 heroku 即可部署:

```
git add . 
git commit -m "deploy app"
git push heroku main
```

## 三、自定义域名与 SSL

若要添加自定义域名和 SSL,需要将 Dyno 类型设置为 Basic 或以上。

### 1. 更改 Dyno 类型

查看当前类型:

```
heroku ps:type -a serverstatus-rust-heroku 
```

设置为 Basic 型:

```
heroku ps:type basic -a serverstatus-rust-heroku
```

### 2. 添加域名

在 Heroku Dashbord 的 `Settings` - `Domain` 中添加域名,并将生成的 DNS Target 添加到域名的 CNAME 中。

访问 http://域名.com 验证是否生效。

![domain](https://github.com/kissyouhunter/ServerStatus-Rust/raw/heroku/heroku/readme-pics/domains.jpg)

### 3. 开启 SSL

在 `SSL Certificates` 中点击 `Configure SSL`,选择 `Automatic Certificate Management (ACM)`。

现在就是可以通过 HTTPS 访问探针了。

![ssl](https://github.com/kissyouhunter/ServerStatus-Rust/raw/heroku/heroku/readme-pics/ssl.jpg)

## 四、后续操作

- 每次修改 config.toml 后需要 push 一次代码以应用更改。

```bash
git add . && git commit -m "update" && git push heroku main
```

- 如果使用 1.7.2 部署不成功,尝试 1.7.1 或其他版本。