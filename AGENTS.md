# LightTodo 项目专属规则

通用规则见：`E:\@imFile-Download\AI-Useful-Prompt\通用开发工作规则.md`。

- 本项目是 Tauri + Vue 3 + TypeScript 的 Windows 待办桌面应用，Rust/Tauri 代码在 `src-tauri`。
- 开发命令：`npm run dev`；前端检查/构建：`npm run build`；正式 EXE：获得授权后使用 `npm run build:exe`。
- 项目名与交付映射：`LightTodo -> LightTodo.exe`。
- 最终 Windows 产物只复制到 `D:\@Software\LightTodo\LightTodo.exe`，不要复制 `src-tauri/target`、`dist` 或其他中间产物。
- 汇报最终产物大小时统一使用 MB，按 1 MB = 1024 × 1024 字节换算并保留两位小数。
- 涉及待办数据、提醒、配置和持久化时保持已有本地数据兼容，不因 UI 调整破坏旧数据读取。
- UI 修改未经“11”“构建”或“打包”授权不构建最终 EXE。
