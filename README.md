# 今天吃什么 · WHAT / TO / EAT

一个用 **Dioxus 0.7 + Rust + SQLite** 实验跨平台应用和在线学习的本地优先项目。一次只推荐一个用餐选项：吃，就记录；不吃，就换一个。Linux 桌面已编译启动；安卓已使用 JDK 26 + SDK 37 构建 ARM64 调试 APK 并通过签名检查，尚未完成真机运行验证。

## 运行桌面版

需要 Rust 和平台 WebView 开发库。当前 Linux 环境已具备 GTK 3、WebKit2GTK 4.1 及编译工具：

```sh
cargo run --locked --features desktop --bin whatoeat
```

初次运行没有饮食记录。可自行添加菜单，或点击「加入六个常见选项」。该操作只加菜单，不生成虚假历史。支持单项推荐、吃/不吃、撤销、手动补记与修改、菜单改名/停用/删除、复制导出、粘贴合并。删除菜单项会保留已有饮食记录；改名会同步显示到历史记录。操作成功提示悬浮显示，3 秒后自动消失。

右上角下拉框可直接选择主题：原味海报、罗德岛、莱茵生命、企鹅物流、卡西米尔。主题即时切换并在本机保存，重启后沿用；清空或导入饮食记录不会重置主题。四套明日方舟风格主题使用配色和 CSS 图形表现。

桌面数据使用操作系统的本地应用目录（Linux 默认 `~/.local/share/whatoeat/whatoeat.sqlite3`）。测试时可以指定隔离目录：

```sh
WHATOEAT_DATA_DIR=/tmp/whatoeat-demo cargo run --features desktop --bin whatoeat
```

## 安卓开发环境

详细配置见 [Android 开发说明](docs/android.md)。需要 Android SDK、NDK、JDK、Rust 安卓目标及 Dioxus CLI。推荐通过 Android Studio 的 SDK Manager 安装工具；写 Rust 代码仍可使用现有编辑器。

```sh
cargo install dioxus-cli --version 0.7.10 --locked
rustup target add aarch64-linux-android
# 使用当前 JDK、SDK 37 和已安装的 NDK 构建：
python3 scripts/build_android.py
# 连接已开启 USB 调试的手机后：
adb install -r target/android-build/whatoeat-debug.apk
```

手机与桌面各自保存，不自动同步。在「复制与合并」页面点「复制当前记录」，把文本粘贴保存；导入时粘贴文本，点「合并记录」即可，不再选择文件或覆盖整库。

- 菜单以名称匹配，忽略首尾空格和英文字母大小写，导入的名称及启用/删除状态覆盖同名本地项；未出现的本地项保留。
- 食用时间保存到分钟，按「名称 + 分钟」匹配；重复项以导入内容为准，其他分钟的本地记录保留。重复导入不会重复记餐或学习。导入包含的记录会覆盖同位置的本地删除或撤销状态。
- 兼容旧版 JSON 备份。新的交换格式没有内部 ID、模型快照或页面状态，保留重新学习所需的反馈事实。
- 「清空全部数据」经页面确认后删除本机菜单、餐次、学习反馈和旧请求记录；已有外部备份不受影响。
- 数据库升级和合并前自动把原始数据保存在同目录的 `recovery/`。首次启动自动升级旧库，保留改名、历史和撤销关联。

数据库为数据目录下的 `whatoeat.sqlite3`。当前 Linux 默认路径为 `~/.local/share/whatoeat/whatoeat.sqlite3`；安卓为应用私有 `files/whatoeat.sqlite3`，通常位于 `/data/user/0/app.whatoeat.local/files/whatoeat.sqlite3`。设置 `WHATOEAT_DATA_DIR` 时以该目录为准。

## 验证

```sh
cargo test --locked --no-default-features --lib --tests
cargo clippy --locked --features desktop --bin whatoeat -- -D warnings
cargo fmt --all --check
```

核心测试覆盖冷启动、两参数学习、拒绝去重、撤销重建、修改补录、并发重复提交、持久化、名称合并、记录去重、旧库迁移和导入事务。模拟用户验证只说明模型能从合成反馈学习，不证明真实偏好会按该曲线收敛。

可用真实组件生成静态布局预览（显式示例数据、不连接数据库、按钮无交互）：

```sh
cargo run --features preview --bin preview > /tmp/whatoeat-preview.html
# 菜单页及悬浮提示布局：
WHATOEAT_PREVIEW_SCREEN=foods WHATOEAT_PREVIEW_TOAST=1 cargo run --features preview --bin preview > /tmp/whatoeat-menu-preview.html
# 罗德岛主题（也支持 poster / rhine / penguin / kazimierz）：
WHATOEAT_PREVIEW_THEME=rhodes cargo run --features preview --bin preview > /tmp/whatoeat-theme-preview.html
```

## 代码导航

- `src/model.rs`：所选方案 B，两参数 logistic 模型。
- `src/domain.rs`：决策状态机、反馈、历史、撤销与重建。
- `src/store.rs`：SQLite 事务、命令去重、迁移/合并前备份。
- `src/transfer.rs`：无 ID 的交换格式、按名称合并和记录去重。
- `src/platform.rs`：桌面/安卓数据目录与剪贴板。
- `src/ui.rs`、`assets/app.css`：Dioxus RSX 与响应式海报风格界面。
- [架构](docs/architecture.md) · [算法与交互](docs/design.md) · [模型对比](docs/algorithm-options.md)。

这是个人实验版：数据库目前保存完整状态快照，每次反馈重放历史，适合小规模个人数据。安卓剪贴板、返回键和进程恢复仍需真机验收；Windows/macOS 尚未构建。没有云服务、账号系统、后台学习任务或正式发布签名。
