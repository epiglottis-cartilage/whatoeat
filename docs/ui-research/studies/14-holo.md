# HO-01：Android 4 Holo → Holo 日常界面

## 选取与分析

Android 官方 [Holo Everywhere](https://android-developers.googleblog.com/2012/01/holo-everywhere.html)，2012-01-03，文章中 Theme.Holo 的 [设置页截图](../references/holo/HO-01-widgets.png)。黑灰渐变面、青蓝色操作栏细线、灰色分组标题与下划线、薄分隔列表和直角开关共同构成层级；主要内容没有大圆角卡片和悬浮阴影。截图状态栏属于系统，不迁移为应用里的假状态。

## 实施

名称「Android 4 · Holo」，key `holo`。使用黑灰底、青蓝细线、轻字重标题、紧凑列表和下划线输入框。宽屏概览与决策并排，手机概览压缩成分组标题和统计行。按钮是直角灰色按键，选中项有青色底边；导航保持已有固定底部位置，不冒充系统导航键。设置页开关的明暗分区迁移为菜单推荐状态按钮，但文字仍说明真实作用。

字体优先 Roboto，设备缺字回退至中文系统字体。它是 Dioxus/CSS 对历史 Holo 视觉的近似，不更换 Android Manifest 主题，也不要求设备运行 Android 4。按压反馈是色块高亮而非大幅位移；保留减少动态效果支持。

## 验证

28 个 Chromium 场景、4 项主题偏好测试通过，审阅 [桌面](../previews/holo/desktop.png)、[手机](../previews/holo/mobile.png)，[检查报告](../previews/holo/validation.json)。固定导航、页脚、通知、长文本与减少动态效果通过静态布局检查；没有执行 Android 真机验收。
