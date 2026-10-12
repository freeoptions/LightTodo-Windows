# LightTodo 项目专属规则

本项目继承 Codex 已加载的全局 `AGENTS.md`（本机：`C:\Users\freez\.codex\AGENTS.md`）；以下保留项目专属规则。

- 本项目是 Tauri + Vue 3 + TypeScript 的 Windows 待办桌面应用，Rust/Tauri 代码在 `src-tauri`。
- 开发命令：`npm run dev`；前端检查/构建：`npm run build`；正式 EXE：获得授权后使用 `npm run build:exe`。
- 项目名与交付映射：`LightTodo -> LightTodo.exe`。
- 最终 Windows 产物只复制到 `D:\@Software\LightTodo\LightTodo.exe`，不要复制 `src-tauri/target`、`dist` 或其他中间产物。
- 汇报最终产物大小时统一使用 MB，按 1 MB = 1024 × 1024 字节换算并保留两位小数。
- 涉及待办数据、提醒、配置和持久化时保持已有本地数据兼容，不因 UI 调整破坏旧数据读取。
- UI 修改未经“11”“构建”或“打包”授权不构建最终 EXE。

## 构建与共享缓存约束

- 2026-10-11 已按用户授权把编译缓存移至 `E:\BuildCaches\projects\LightTodo\cargo-target`。本项目 `src-tauri\target` 与原 `D:\@BuildCache\LightTodo-codex-release` 都是指向该目录的目录链接；默认 Cargo 路径和历史命令均复用这份缓存，不重建 D 盘实体缓存。已用离线、锁定依赖的 `cargo metadata --no-deps` 确认默认 target 实际解析到 E 盘；此验证不等于 Release 构建通过。
- 依赖安装、编译检查或打包前，必须主动读取并遵守 D:\MyCodes\@UploadToGithub\AGENTS.md（共享规则），不依赖仓库外父目录自动加载；本项目已有的构建入口、交付方式及更严格限制继续适用。
- Windows 最终 Release 构建须有“11”“构建”“打包”等明确授权，并使用本项目既有正式入口；普通修改只做已允许的必要检查，前端构建或编译检查不等于最终打包授权。构建授权不自动包含启动 GUI、安装或发布。
- 使用 npm、pip、uv 时，下载缓存分别使用 E:\BuildCaches\npm、E:\BuildCaches\pip、E:\BuildCaches\uv，不改回 C 盘或重建旧入口。其他工具沿用已确认的有效位置；BUILD_CACHE_PLAN.md 中尚未实施的规划不视为已迁移。不同工具使用各自缓存子目录。
- 执行前核对工具实际缓存配置及当前进程环境，特别是 GRADLE_USER_HOME、CARGO_HOME、RUSTUP_HOME、APPDATA、LOCALAPPDATA；继承异常时，只为当前命令补齐经核实的现有配置，不擅改全局环境、迁移缓存或新建替代缓存目录。
- 复用现有依赖和增量编译缓存；只有依赖变化、缺失或确认损坏时才处理对应部分，不盲目重装全部依赖、清缓存或全量重建。不执行 cargo clean，不擅删有效 Gradle/Release 缓存；node_modules、.venv、项目 .gradle 和编译输出继续按项目隔离。
- 只执行本次任务必要的检查，不顺带构建其他项目或无关模块；通过后没有新修改或新问题，不重复全量检查。较大下载或构建前核对源码、缓存及产物所在盘的可用空间和实际输出路径，空间不足先说明，不靠清空有效缓存解决。
- 历史正式打包副本只保留最近一次成功且验证完整的版本，固定交付位置的当前版本另行保留；新版验证并交付成功后再按共享规则及本项目清理例外处理旧副本，失败保留上一成功版本。源码、用户修改、运行数据和有效构建缓存不属于历史副本。
