# 跟随系统语言

当前支持简体中文和英文。`zh`（含 zh-HK、zh-TW）使用现有简体文案，`en` 使用英文；按偏好列表选择第一个支持的语言，没有支持项时回退英文。本次没有新增语言选择菜单或数据库设置。

## 语言来源

- Android：通过 JNI 读取 `java.util.Locale.getDefault().getLanguage()`；原生读取失败才采用 WebView 报告的语言。
- 桌面：优先读取进程环境中的 `LC_ALL`、`LC_MESSAGES`、`LANGUAGE`、`LANG`，按上述顺序采用首个非空值；没有值时采用 WebView 的 `navigator.languages` / `navigator.language`。
- 启动、WebView `languagechange` 及页面恢复可见时重新检查。Android 系统也可能因配置变化重建应用。桌面进程环境不会随设置自动更新，因此更改桌面系统语言后可能需要重启应用。
- 页面根节点设置正确的 `lang`，窗口文档标题也翻译。日期记录仍保持分钟精度；数字式历史日期格式不变，日期输入框遵循平台控件表现。

语言在内存中解析，不写入备份；切换语言不会重建学习模型、翻译已有菜名或改变历史。英文环境首次加入示例选项时使用英文名称；与中文同义的英文菜名仍按既有“名称为依据”的规则视为不同菜单项，不做语义合并。

## 实现入口

- `src/i18n.rs`：偏好解析、静态文案查找、命名占位符、通知与错误翻译。
- `assets/locales/en.json`：以现有中文文案为 key 的英文词典；不调用在线翻译服务。
- `src/platform.rs`：设备语言；`src/ui.rs`：响应式语言状态与渲染。
- `assets/locales/layout.css`：英文长文本的小屏排版调整。

业务层保存原始通知，显示时翻译；嵌套应用错误前缀可以逐层翻译，外部数据库、系统错误的诊断详情保留原文。食物名称作为用户数据直接渲染，不送入词典。

新增文案须同时加入词典。未知 key 会回退原文，测试会检查占位符一致性，浏览器检查会检查应用自有文字中的中文漏译；这不能替代人工审阅新增页面。

## 验证

```sh
cargo test --features preview
cargo clippy --features desktop,preview --all-targets -- -D warnings
cargo build --features preview --bin preview
# 静态预览，不读真实数据库
WHATOEAT_PREVIEW_LANG=en WHATOEAT_PREVIEW_THEME=holo target/debug/preview > /tmp/whatoeat-en.html
# 使用已安装 Playwright 的目录
node scripts/check_ui.cjs --playwright /path/to/playwright --language en --output /tmp/ui-en
node scripts/check_ui.cjs --playwright /path/to/playwright --language zh-CN --output /tmp/ui-zh
python3 scripts/build_android.py --release
```

Rust 测试共 43 项通过，包括语言优先级、区域语言标签、嵌套错误、占位符及英文示例菜单的幂等性；all-targets Clippy 通过。19 套主题每种语言 402 个 Chromium 布局场景，覆盖四页面、360–1060px、极端内容、通知、固定导航与减少动态效果。最终报告保存在 [研究回归目录](ui-research/previews/regression/)。

Android release 使用现有 JDK 26、SDK 37、NDK 30，ARM64 APK 为 3,474,800 字节（3.31 MiB），完成 ZIP 内容、16KB 对齐和签名检查。产物为 `target/android-build/whatoeat-release.apk`，本地调试证书签名；SHA-256 为 `54b20685ddda58f2be1213cb27c2d043d1bd458225128a8110072a9871c03329`。

边界：浏览器场景使用 Dioxus SSR，不执行真实业务点击。尚未完成 Android 真机切换系统语言、后台恢复与本地输入法验证；也未验证 Windows/macOS 的实际系统语言报告。字体研究的候选未嵌入，此次布局检查针对运行版已有系统回退字体。
