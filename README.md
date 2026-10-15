# WhiteClean

给普通 Mac 用户的一键清理。软件先把能删的挑好，列出具体文件；你确认之后，才会放进废纸篓。

当前测试包面向 Apple 芯片（M 系列）。

## 下载

预发布安装包在 [Releases](https://github.com/luckySpro/whiteClean/releases)。下载 DMG 后，把 WhiteClean 拖进「应用程序」。

这个测试包没有 Apple 公证。如果系统提示无法打开，到「系统设置 → 隐私与安全性」里选择仍要打开。

## 它会做什么

检查范围是下载、桌面、文稿，以及可以重新生成的缓存和日志。建议默认勾上，想留下的取消勾选即可。每一项旁边有放大镜，可以在访达里打开真实位置。

- **重复文件按内容比对。** 名字可以不同。大小相同的文件会再比一遍内容，只有字节完全一致才算重复。同一份留下最新的，其余建议删掉。硬链接和系统克隆出来的同一份数据不会被当成两份。小于 5 MB 的文件不进入这条建议。
- **同一文件夹里的旧版本。** `报告.pdf` 和 `报告 副本.pdf`、`报告 (1).pdf`、`报告-v2.pdf`，以及带「拷贝」「copy」的文件，会收成一棵树。上面标「留下」的是较新的那份，下面挂着旧的。两份内容不一样时会写明。另一个文件夹里碰巧同名的文件不会配成一对。
- **过期安装包。** 下载或桌面上放了一个月以上的 `.dmg`、`.pkg`、`.mpkg`、`.iso`、`.xip`。
- **缓存和日志。** 只看 `~/Library/Caches`、`~/Library/Logs`，以及 Chrome、Edge、Brave、Firefox 里的 Cache、Code Cache、GPUCache、cache2。同一应用收成一个文件夹。Cookies、浏览历史、书签、登录数据和密钥不会动。

看起来像合同、证件、密码或密钥的文件会单独列出来，不会被勾上。

确认之后文件进入废纸篓。清空废纸篓之前，可以一键撤销。

## 本地运行

需要 Rust 稳定版、Node.js 22 和 macOS。

```sh
npm --prefix apps/desktop install
npm --prefix apps/desktop run tauri dev
```

```sh
cargo test --workspace
```

推送到 `main` 会在 GitHub Actions 上编译 Apple 芯片的 DMG，并放到预发布。

## 许可

MIT。M&J Group &lt;473276@qq.com&gt;。
