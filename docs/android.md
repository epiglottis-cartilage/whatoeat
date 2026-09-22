# Android 构建：保留 JDK 26，使用 SDK 37

使用本机现有 OpenJDK 26.0.2.1、Android SDK 37.0、NDK r30 和 Dioxus CLI 0.7.10。Rust 代码原生编译，界面由 Android WebView 渲染，不需要 Node/npm 或 WASM 目标。

## 当前工具链

| 组件 | 版本 / 路径 |
| --- | --- |
| JDK | OpenJDK 26.0.2.1，`/usr/lib/jvm/java-26-openjdk` |
| Android Studio | `/opt/android-studio` |
| Android SDK | `~/Android/Sdk`，Platform 37.0 |
| SDK Build-Tools | 36.0.0 |
| NDK | r30，`~/Android/Sdk/ndk/30.0.16248370` |
| SDK Command-line Tools | 已安装于 `cmdline-tools/latest` |
| Dioxus CLI | 0.7.10 |
| Rust 目标 | `aarch64-linux-android` |
| Gradle / Android Gradle Plugin | 项目生成工程使用 9.5.0 / 9.3.3 |
| 设备 | 上次检查没有连接手机或配置 AVD |

## 构建

在项目根目录运行：

```sh
# 日常安装：优化后的 release 包
python3 scripts/build_android.py --release
# 开发调试：保留调试符号
python3 scripts/build_android.py
```

两种模式均为 ARM64。release 默认沿用本机 Android 调试证书，方便自用安装和覆盖同证书的 debug 包；应用自身关闭调试。这不是正式发布签名。需要使用自己的发布证书时，运行 `python3 scripts/build_android.py --release --unsigned`，然后自行签名；未签名 APK 不能直接安装。已有全部依赖缓存时，可加 `--offline`。

`--offline` 控制 Cargo 依赖解析和适配后的 Gradle 打包。Dioxus 0.7.10 内部首次尝试运行旧模板 Gradle 时不转发该选项，因此它不是整个脚本的网络隔离开关。

脚本保留现有 `JAVA_HOME`；未设置时使用 PATH 中 `java` 的真实路径。SDK 默认取 `~/Android/Sdk`，NDK 优先使用 `NDK_HOME` / `ANDROID_NDK_HOME`，否则选择 SDK 下安装的最高版本。不修改系统 JDK、shell 配置或已安装 Dioxus CLI 的模板。

需要显式设置时：

```sh
export JAVA_HOME="/usr/lib/jvm/java-26-openjdk"
export ANDROID_HOME="$HOME/Android/Sdk"
export NDK_HOME="$ANDROID_HOME/ndk/30.0.16248370"
export ANDROID_NDK_HOME="$NDK_HOME"
python3 scripts/build_android.py
```

结果与日志（均位于 `target/android-build/`）：

| 模式 | APK | 编译 / 打包日志 |
| --- | --- | --- |
| 默认 debug | `whatoeat-debug.apk` | `dioxus.log` / `gradle.log` |
| `--release` | `whatoeat-release.apk` | `dioxus-release.log` / `gradle-release.log` |
| `--release --unsigned` | `whatoeat-release-unsigned.apk` | 同 release |

每个 APK 附有同名 `.json`，记录字节数、原生库体积、SHA-256、签名方式及 ABI。校验日志为 `verify-debug.log` / `verify-release.log`。构建失败时不会用旧原生库继续打包，也不会覆盖上次成功导出的 APK。

首次构建需要下载 Gradle 和 Android 依赖。脚本将现有、不带认证的 HTTP 代理地址从 `http_proxy` / `https_proxy` 转给 Java；也可通过已有 `JAVA_OPTS` 配置 Java 网络参数。

## 为什么增加适配脚本

Dioxus 0.7.10 每次构建都会重写生成目录中的 Gradle 文件。内置模板固定为 Gradle 9.1.0、AGP 8.7.0、Kotlin 插件 2.0.20，无法直接作为当前 JDK 26 / SDK 37 的已验证组合。

脚本先调用 dx 完成 Rust 编译与平台工程生成。只有错误明确发生在 APK assembly 阶段时才进入兼容处理；Rust 编译、资源或工程生成错误会直接退出，避免用旧库掩盖失败。随后只修改 `target/` 中的生成文件并重新运行 Gradle：

- Gradle 9.5.0 支持运行在 JDK 26 上。
- AGP 9.3.3 支持 API 37，并使用现有 Build-Tools 36.0.0。
- 使用 SDK 37.0 对应的 `release(37) { minorApiLevel = 0 }` 配置。
- 迁移到 AGP 内置 Kotlin，移除旧 Kotlin 插件与过期属性，注册 Kotlin 源码目录。
- 将旧 Manifest 的原生库解压属性迁移到 Gradle `packaging.jniLibs.useLegacyPackaging`。
- Java/Kotlin 的应用字节码目标仍为 17；这不改变运行 Gradle 和编译器的 JDK 26。

直接运行 `dx build` / `dx serve` 会重新写回旧模板，因此目前推荐用上述脚本构建，再用 adb 安装。本脚本是针对 0.7.10 的临时适配；升级 Dioxus 时需重新验证。

## Release 体积优化

- `Cargo.toml` 的独立 `android-release` profile 使用 `opt-level = "s"`、Thin LTO、单代码生成单元、不生成调试信息及符号裁剪。桌面 release 配置不变；保留默认 panic 行为。
- `dx --release` 负责优化 Rust 和收集资源，脚本随后显式执行 `:app:assembleRelease`。Dioxus 0.7.10 在未提供发布签名配置时，单独使用 `dx --release` 仍可能组装 debug APK。
- release 启用 R8 代码优化与资源裁剪，保留 Dioxus/Wry 生成的 JNI 和 WebView 桥接规则。`scripts/android-release.pro` 补充 Wry 0.53.5 中由 Rust 按名称调用的两个 WebView 方法。
- 仅打包 `arm64-v8a`，原生库继续使用 ZIP 压缩。符号裁剪由 Dioxus 在资源提取后执行，不能提前对 Rust 产物手动 strip。
- 导出前检查 ZIP 内容与 CRC、16 KiB 页对齐要求以及已签名 APK 的签名。首次 release 会比 debug 多下载 Lint 等依赖，Rust 优化和 R8 也需要更多时间。

构建脚本的回归检查不依赖 Android 工具链：

```sh
python3 -m unittest discover -s scripts -p 'test_build_android.py' -v
```

## 本次构建结果

2026-09-22：同一份源码完成 debug 与 release 构建，JDK 仍为 26.0.2.1，SDK 仍为 37.0。体积为本次实测，后续修改可能改变结果：

| 产物 | debug | release |
| --- | ---: | ---: |
| APK | 65.94 MiB（69,139,020 字节） | 3.25 MiB（3,409,752 字节） |
| Rust 原生库（解压后） | 229.93 MiB | 7.63 MiB |
| Rust 原生库（APK 内压缩后） | 59.72 MiB | 2.45 MiB |

APK 体积减少 **95.07%**。这包含 Rust 优化与去除调试符号的收益，不代表安装后数据目录占用。

- debug/release APK Signature Scheme v2、ZIP CRC 和对齐检查通过。
- `--release --unsigned --offline` 实际构建通过，确认输出无签名；默认 debug/release 的证书一致。
- release Manifest 未开启调试，ELF 保留 `main`、`start_app` 及全部 20 个 JNI 导出，移除调试段和静态符号表。
- R8 seeds、mapping 与最终 DEX 检查确认 WebView/IPC 桥接名称保留；5 个 CSS、3 个 SVG 的完整字节仍嵌入原生库。
- 10 项构建脚本回归测试通过，覆盖构建模式、签名切换和失败时不发布旧产物。
- 包名 `app.whatoeat.local`，compileSdk/targetSdk 37，minSdk 24。
- 包内包含 `lib/arm64-v8a/libmain.so`、DEX 和启动 Activity。

## 安装与验证

手机开启开发者选项和 USB 调试，并确认电脑的调试授权：

```sh
adb devices -l
adb install -r target/android-build/whatoeat-release.apk
adb shell am start -n app.whatoeat.local/dev.dioxus.main.MainActivity
```

目前最低系统版本设为 API 24，目标和编译 SDK 为 37。生成 APK 不等于真机运行通过；需要检查首次启动、吃/不吃、进程恢复、撤销和修改、离线使用、备份导入导出、键盘、返回键与系统 WebView。

安卓数据库保存在应用私有 `filesDir/whatoeat.sqlite3`，通常为 `/data/user/0/app.whatoeat.local/files/whatoeat.sqlite3`。导出通过 JNI `ClipboardManager` 复制精简 JSON 文本，导入通过文本框粘贴后按名称增量覆盖、按名称与食用分钟合并。剪贴板和 JNI 平台行为仍须真机验收。默认 debug/release 包均使用本机调试证书；正式发布签名尚未配置。

参考：[Gradle Java 兼容矩阵](https://docs.gradle.org/current/userguide/compatibility.html#java_runtime)、[AGP 9.3 兼容要求](https://developer.android.com/build/releases/agp-9-3-0-release-notes)、[迁移到内置 Kotlin](https://developer.android.com/build/migrate-to-built-in-kotlin)。

优化依据：[Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)、[Android 代码与资源优化](https://developer.android.com/topic/performance/app-optimization/enable-app-optimization)。
