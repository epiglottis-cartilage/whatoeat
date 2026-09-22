# 中英文字体选型

研究日期：2026-09-23。范围覆盖当前 14 套完整界面与 5 套保留配色。这是字体选型交付：下表字体尚未下载、子集化或捆绑进应用；当前运行版仍使用各主题已有的系统字体回退。英文适配已另外实施。

字体搭配是本项目的设计判断，不是对游戏原字体的鉴定。优先把辨识度放在标题、数字与短标签上，输入框、长菜名、错误提示保持易读。中文使用真正覆盖中文的字体，不把日文字库当作完整中文替代。

## 逐主题方案

| key / 主题 | 英文标题 / 正文 / 数字 | 中文搭配 | 使用理由与限制 |
| --- | --- | --- | --- |
| poster 原味海报 | Archivo Black / IBM Plex Sans / Plex Mono | Noto Sans CJK SC 900 / 400 | 大黑标题保留海报力量；正文不使用超粗体 |
| rhodes 冰蓝终端 | Barlow Condensed / Plex Sans / Plex Mono | Noto Sans CJK SC | 收紧英文标题，数值形成终端刻度；不改变现有配色布局 |
| rhine 苔绿纸页 | Cormorant Garamond / Plex Sans / Plex Mono | Noto Serif CJK SC 标题、Sans 正文 | 纸页标题有印刷感；较小提示仍用黑体 |
| penguin 赤橙速递 | Archivo Black / Plex Sans / Plex Mono | Noto Sans CJK SC 900 / 400 | 粗重快递标签；保持按钮完整可读 |
| kazimierz 鎏金夜幕 | Cormorant Garamond / Plex Sans / Plex Mono | Noto Serif CJK SC 标题、Sans 正文 | 衬线标题与金色契合；细笔画不用于小号文字 |
| control 罗德岛中枢 | Barlow Condensed / Plex Sans / Plex Mono | Noto Sans CJK SC | 工业窄标题与清晰信息层级；不能靠缩放压扁中文 |
| reclamation 生息演算 | Barlow Condensed / Plex Sans / Plex Mono | Noto Sans CJK SC | 工具面板、营地清单，保留硬朗数字 |
| expedition 集成战略 | Cormorant Garamond / Plex Sans / Plex Mono | Noto Serif CJK SC 标题、Sans 正文 | 旅程、书页和章节感，表单维持无衬线 |
| automata 尼尔 | IBM Plex Sans / Plex Sans / Plex Mono | Noto Sans CJK SC | 克制的档案排版；英文大写短标签可加字距，中文不照搬 |
| phantom P5 | Archivo Black / Plex Sans / Plex Mono | Noto Sans CJK SC 900 / 400 | 巨大粗黑标题适合剪贴块；倾斜由容器承担，不伪造字体斜体 |
| island 动森 | Nunito / Nunito / Nunito | LXGW WenKai 短标题、Noto Sans 正文 | 圆润西文搭配手写便签；长说明不用细手写体 |
| terminal 辐射 | VT323 / IBM Plex Mono / Plex Mono | Fusion Pixel 12px 标题、Noto Sans 正文 | CRT 短标题用像素字，菜单说明用清晰等宽西文；VT323 不覆盖中文 |
| strand 死亡搁浅 | Barlow Condensed / Plex Sans / Plex Mono | Noto Sans CJK SC | 清单标签、运单数值；不声称等同原作定制字形 |
| frontline 战地 1 | Barlow Condensed / Barlow / Barlow | Noto Sans CJK SC | 窄长大写菜单与轻字重层级；避免把正文全部变窄 |
| marathon 马拉松 | Cormorant Garamond / Space Grotesk / Plex Mono | Noto Serif CJK SC 标题、Sans 正文 | 按本轮 2026 参考图保留衬线标题与协议数字的反差 |
| valley 星露谷 | Fusion Pixel 12px proportional / Nunito / Fusion Pixel | Fusion Pixel zh-Hans 短标题、LXGW WenKai 便签 | 中英像素统一；24px 标题先试整数放大，长记录仍用可读正文 |
| ink 喷射战士 | Archivo Black / Nunito / Nunito | Noto Sans CJK SC 900 / 400 | 重心饱满的墨块标题与圆润正文；是开源替代，不是任天堂字体 |
| classic 经典 Mac | Fusion Pixel proportional / Plex Sans / Plex Mono | Fusion Pixel zh-Hans 标题、Noto Sans 正文 | 位图标题增强早期窗口感；它不是 Chicago 原字体，小表单不强行像素化 |
| holo Android 4 | Roboto 2 Light / Regular / Medium | Noto Sans CJK SC / 系统中文无衬线 | Roboto 字族符合 Holo 方向；2.x 不是 Android 4.0 初版字形的逐字复刻 |

## 可追溯来源与许可

以下链接指向维护者或 Google Fonts 字体仓库。许可按具体仓库核对，不能只看“Google 字体”标签判断。未来若嵌入，须固定提交或发行版、记录文件 SHA-256，并随字体保留对应版权与许可文件。

| 字体 | 获取 / 预览来源 | 已核对许可 | 建议字重 / 字符范围 |
| --- | --- | --- | --- |
| Barlow、Barlow Condensed | [作者仓库](https://github.com/jpt/barlow) | [OFL-1.1](https://github.com/jpt/barlow/blob/main/OFL.txt) | 300、400、600；先取拉丁字符，中文回退 |
| IBM Plex Sans、Mono | [IBM 官方仓库](https://github.com/IBM/plex) | [OFL-1.1；Plex 为保留字体名](https://github.com/IBM/plex/blob/master/LICENSE.txt) | 400、600；数值 Mono、正文 Sans |
| Archivo Black | [Google Fonts 字族目录](https://github.com/google/fonts/tree/main/ofl/archivoblack) | [OFL-1.1](https://github.com/google/fonts/blob/main/ofl/archivoblack/OFL.txt) | 固有超粗字形，仅标题 |
| Nunito | [Google Fonts 字族目录](https://github.com/google/fonts/tree/main/ofl/nunito) | [OFL-1.1](https://github.com/google/fonts/blob/main/ofl/nunito/OFL.txt) | 400、700、900；拉丁字符 |
| Cormorant Garamond | [Google Fonts 字族目录](https://github.com/google/fonts/tree/main/ofl/cormorantgaramond) | [OFL-1.1](https://github.com/google/fonts/blob/main/ofl/cormorantgaramond/OFL.txt) | 500、600；大标题，不用于 12px 提示 |
| Space Grotesk | [Google Fonts 字族目录](https://github.com/google/fonts/tree/main/ofl/spacegrotesk) | [OFL-1.1](https://github.com/google/fonts/blob/main/ofl/spacegrotesk/OFL.txt) | 400、600；拉丁正文与模块名称 |
| VT323 | [Google Fonts 字族目录](https://github.com/google/fonts/tree/main/ofl/vt323) | [OFL-1.1](https://github.com/google/fonts/blob/main/ofl/vt323/OFL.txt) | Regular；短英文终端标签 |
| Roboto 2 | [历史维护仓库](https://github.com/googlefonts/roboto-2) | [Apache-2.0](https://github.com/googlefonts/roboto-2/blob/main/LICENSE) | 300、400、500；此许可结论只针对该仓库，不外推新版 Roboto |
| Noto Sans CJK、Noto Serif CJK | [Noto CJK 官方仓库](https://github.com/notofonts/noto-cjk) | [Sans OFL-1.1](https://github.com/notofonts/noto-cjk/blob/main/Sans/LICENSE)、[Serif OFL-1.1](https://github.com/notofonts/noto-cjk/blob/main/Serif/LICENSE) | SC 地区字形；正文 Sans 400、标题 Sans 700/900 或 Serif 500/600 |
| LXGW WenKai / 霞鹜文楷 | [作者仓库与示例](https://github.com/lxgw/LxgwWenKai) | [OFL-1.1](https://github.com/lxgw/LxgwWenKai/blob/main/OFL.txt) | Regular / Medium；先用于便签与短句 |
| Fusion Pixel / 缝合像素 | [作者仓库与在线预览](https://github.com/TakWolf/fusion-pixel-font) | [字体 OFL-1.1](https://github.com/TakWolf/fusion-pixel-font/blob/master/LICENSE-OFL)；构建程序另为 MIT | 12px proportional、zh-Hans；上游建议一般排版优先比例模式 |

OFL 字体发布时附原版权和许可证，子集化或改名还需遵循相应保留名要求；Roboto 2 附 Apache 许可及适用的 NOTICE。这里不分发字体文件，因此并未将这些许可变为本项目代码许可。

## 回退与体积策略

1. 西文装饰标题可使用小型离线 WOFF2 子集；同字族跨主题复用。应用启动不访问 Google Fonts CDN，断网时不能出现空白文字。
2. 中文先用系统：`"Noto Sans CJK SC", "Noto Sans SC", "PingFang SC", "Microsoft YaHei", sans-serif`。衬线标题：`"Noto Serif CJK SC", "Songti SC", "SimSun", serif`。这些都是查找顺序，不保证设备装有每一种字体。
3. 用户可以录入任意菜名，不能只把当前词典字符当作完整覆盖。若未来捆绑中文子集，保留系统字体作为最后回退；不能为缩包截掉未知汉字。
4. Holo 优先系统 Roboto；桌面没有 Roboto 时仍允许无衬线回退。完整 CJK 字库先不加入 APK。本次字体研究增加的运行包体积为零。
5. 可先落地 Barlow Condensed、Archivo Black、Nunito、Plex Mono 四种西文，再验证衬线和像素体。每次应量测实际 WOFF2 压缩后增量，不能把字族网页下载量当成 APK 增量。

## 字体落地时的验收样本

中英都测：`牛肉面 / Beef noodles`、`参考周期 7.0 天 / Interval: 7.0 days`、`2026-09-23 01:30`、`复制并合并 / Copy & merge`、`鱼香肉丝盖饭（少辣加蛋）`。另外测试 `饺子 Dumplings` 混排及生僻菜名的系统回退。

在 360、390、760、1060px 检查同一组件；表单至少保留正常字号，不把窄字体当作解决溢出的唯一方法。验收必须确认实际加载字体名称，只有 `font-family` 声明不代表字体已加载。当前 804 个中英布局检查验证的是现有系统回退，不能替代上述候选字体嵌入后的验证。
