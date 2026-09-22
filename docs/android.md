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
python3 scripts/build_android.py
```

脚本保留现有 `JAVA_HOME`；未设置时使用 PATH 中 `java` 的真实路径。SDK 默认取 `~/Android/Sdk`，NDK 优先使用 `NDK_HOME` / `ANDROID_NDK_HOME`，否则选择 SDK 下安装的最高版本。不修改系统 JDK、shell 配置或已安装 Dioxus CLI 的模板。

需要显式设置时：

```sh
export JAVA_HOME="/usr/lib/jvm/java-26-openjdk"
export ANDROID_HOME="$HOME/Android/Sdk"
export NDK_HOME="$ANDROID_HOME/ndk/30.0.16248370"
export ANDROID_NDK_HOME="$NDK_HOME"
python3 scripts/build_android.py
```

结果与日志：

- `target/android-build/whatoeat-debug.apk`：成功后复制出的 ARM64 调试包。
- `target/android-build/dioxus.log`：Rust 编译、资源收集与 Dioxus 工程生成日志。
- `target/android-build/gradle.log`：新版工具链的 APK 打包日志。

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

## 本次构建结果

2026-09-22：使用上述脚本完成构建，`BUILD SUCCESSFUL`。JDK 仍为 26.0.2.1，SDK 仍为 37.0，无需降级。Rust 库和 JNI 适配已完成 ARM64 编译，Java/Kotlin、资源及 APK 打包全部通过。

- APK 约 64.1 MiB，包含调试符号；不代表发布包体积。
- APK Signature Scheme v2 验证通过，ZIP CRC 检查通过。
- 包名 `app.whatoeat.local`，compileSdk/targetSdk 37，minSdk 24。
- 包内包含 `lib/arm64-v8a/libmain.so`、DEX 和启动 Activity。
- 默认旧模板实际报 `Unsupported class file major version 70`；适配后的 Gradle 9.5 构建成功。

## 安装与验证

手机开启开发者选项和 USB 调试，并确认电脑的调试授权：

```sh
adb devices -l
adb install -r target/android-build/whatoeat-debug.apk
adb shell am start -n app.whatoeat.local/dev.dioxus.main.MainActivity
```

目前最低系统版本设为 API 24，目标和编译 SDK 为 37。生成 APK 不等于真机运行通过；需要检查首次启动、吃/不吃、进程恢复、撤销和修改、离线使用、备份导入导出、键盘、返回键与系统 WebView。

安卓数据库保存在应用私有 `filesDir/whatoeat.sqlite3`，通常为 `/data/user/0/app.whatoeat.local/files/whatoeat.sqlite3`。导出通过 JNI `ClipboardManager` 复制精简 JSON 文本，导入通过文本框粘贴后按名称增量覆盖、按名称与食用分钟合并。剪贴板和 JNI 平台行为仍须真机验收。调试包不用于正式发布，发布签名和安装包优化后续处理。

参考：[Gradle Java 兼容矩阵](https://docs.gradle.org/current/userguide/compatibility.html#java_runtime)、[AGP 9.3 兼容要求](https://developer.android.com/build/releases/agp-9-3-0-release-notes)、[迁移到内置 Kotlin](https://developer.android.com/build/migrate-to-built-in-kotlin)。
