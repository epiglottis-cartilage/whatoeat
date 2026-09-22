# 本轮验证记录

日期：2026-09-22。范围是配色显示名与静态界面研究，不是主题重设计验收。

## 实际完成的检查

| 检查 | 结果与边界 |
| --- | --- |
| 真实参考图 | 已实际查看 HOME-01/02、RA-01/02、IS-01；三张社区原图、两张官方视频解码帧；不是生成图或概念重绘 |
| 文件登记 | 5 个唯一 ID；尺寸、字节数、SHA-256 与 `sources.json` 一致；图像合计 3,740,485 字节，约 3.57 MiB |
| 视频时间追溯 | RA-01/02 的分段偏移与归档 HLS 播放列表累计时间一致；仓库只保存帧、播放列表和元数据，没有整段视频 |
| 文档可达性 | 研究入口、三份逐图研究、来源登记、跨界面对照、计划与决策的本地链接均检查存在 |
| 改动边界 | 对照 jj 基线，`assets/app.css`、`src/ui.rs` 及其余既有文件内容保留；既有文件只改 `src/theme.rs` 的名称/描述/说明注释和 `README.md` 的文档说明 |
| 选择兼容 | `Theme` 变体、顺序、历史 key 与解析逻辑保持原样；无需迁移现有主题选择 |
| 偏好回归 | `cargo test --no-default-features --test theme_preferences --offline`：4 项通过；涉及重启持久化、清空/合并/恢复保留主题、未知值回退、已有数据库补充偏好 |
| 格式 | `cargo fmt --all --check` 通过 |
| 编译 | `cargo build --features desktop --bin whatoeat --offline` 通过 |

本轮未重跑完整业务测试或安卓打包；没有修改算法、数据库逻辑、平台代码、布局或动画。已有页面的样式保持不变，不把本次研究描述为新的主题已经完成视觉验收。

## jj 版本锚点

- 研究前基线：change `snlqwlun`，当时 commit `e8c1097f`，描述 `baseline: preserve current Dioxus app before interface research`。
- 本轮研究：change `stywssun`。文档采用稳定 change ID；后续修改会改变 commit ID，不把不断变化的 commit hash 写回同一个提交。
- 研究材料与改名处于同一可审阅工作包；完成后用 `jj new` 留出新的工作 change。本轮没有推送远端。
- RA-02 原帧为 2,020,495 字节，超过 jj 默认新文件快照上限；归档时仅对该次命令设置 `snapshot.max-new-file-size=2020495`，保留原图，不改变全局配置。

可用 `jj log -r snlqwlun::stywssun`、`jj show stywssun --stat` 和 `jj diff --from snlqwlun --to stywssun` 追溯。未来不对包含用户新修改的工作区整体恢复；需要回退时先限定 change 与文件范围。

## 仍然未知的内容

- 截图未证明具体客户端构建号，不能宣称完全代表 2026 年现行 UI。
- 静态样本不提供真实点击目标、可访问性、输入驱动视差与动画时序；没有填写虚构的毫秒或缓动曲线。
- 生息演算参考是日总览与区域地图，活动入口仍未采集；视频外框不属于内层游戏 UI。
- 横屏布局如何迁移到本应用的竖屏、长文本和单候选流程，还要经过下一阶段结构设计。
