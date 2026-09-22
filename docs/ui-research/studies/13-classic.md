# MC-01：经典 Macintosh 桌面 → 开饭窗口

## 选取与分析

[System 1 条目](https://en.wikipedia.org/wiki/System_1) 中的 [历史桌面截图](../references/classic/MC-01-desktop.png)。画面包含黑白点阵桌面、白色菜单栏、黑色窗口轮廓、横线标题栏，以及选中项的黑底白字。前后窗口通过覆盖关系表现层次，没有透明玻璃或模糊阴影。具体捕获环境未知，不以该图断言整个系统的动画行为。

## 实施

名称「经典 Mac · 开饭窗口」，key `classic`。桌面概览与候选是错位的两扇白色窗口；手机改为顺序堆叠。候选使用横线标题栏，主要按钮采用黑色反白，菜单与历史采用紧凑列表。点阵只在窗口外，正文仍是清晰的设备字体；没有添加无功能的关闭、缩放或文件菜单。导航固定、页脚及通知行为沿用已有实现。CSS 硬阴影和标题线为原创，无系统图标拷贝。

## 验证

28 个 Chromium 场景、4 项主题偏好测试通过。审阅 [桌面](../previews/classic/desktop.png) 与 [手机](../previews/classic/mobile.png)，[检查报告](../previews/classic/validation.json)。覆盖四页面、窄/宽屏、长文本、通知、固定导航和减少动态效果；尚未真机验收。
