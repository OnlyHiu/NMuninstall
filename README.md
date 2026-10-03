# NMUninstall

Windows 程序管理与卸载工具 —— 以 Windows 11「设置 → 应用」的原生风格浏览本机已安装程序、查看详情并调用程序自带的卸载器完成卸载。

- **查看程序**：扫描注册表的 64 位与 32 位 `Uninstall` 根，展示与系统「应用」列表一致的字段
- **卸载程序**：解析 `UninstallString`（含 MSI / NSIS / Inno / InstallShield / rundll32 等形态），展示实际将执行的命令再确认
- **残留检测**：卸载后检查残留的注册表键与安装目录，可逐项二次确认后清理
- **Windows 风格**：Fluent 圆角、强调色、Segoe UI 字体、行悬停/选中态、完整键盘操作、深浅色主题

> 完整规格见 [`技术文档.md`](./技术文档.md)。用户手册见 [`docs/用户手册.md`](./docs/用户手册.md)，构建步骤见 [`docs/构建说明.md`](./docs/构建说明.md)。

---

## 特性

| 功能 | 说明 |
|------|------|
| 程序列表 | 名称 / 版本 / 发布者 / 安装日期 / 大小，任意列可排序，搜索框实时过滤 |
| 虚拟化 | `@tanstack/react-virtual`，1000+ 条目稳定滚动 |
| 详情面板 | 图标、安装位置、卸载命令原文、注册表键路径、64/32 位标记 |
| 右键菜单 | 卸载、查看详情、复制卸载命令、打开安装位置、检测残留 |
| 卸载 | 二次确认、展示解析后的命令、静默卸载、5 分钟可配置超时 |
| 残留检测 | 注册表键 + 安装目录，路径白名单 + 符号链接防护 |
| 系统托盘 | 关闭窗口隐藏到托盘，托盘菜单可重新扫描 / 打开日志 / 退出 |
| 国际化 | 简体中文（默认）/ English (US) |
| 主题 | 跟随系统 / 浅色 / 深色 |

---

## 技术栈

| 层级 | 选型 |
|------|------|
| 桌面框架 | Tauri 2 |
| 后端 | Rust（MSVC 工具链） |
| 前端 | React 19 + TypeScript 5.9 + Vite 7 |
| 样式 | Tailwind CSS 4（CSS 变量承载 Win11 设计令牌） |
| 状态 | Zustand 5 |
| 虚拟化 | @tanstack/react-virtual 3 |
| 打包 | Tauri Bundler（NSIS + MSI） |

---

## 快速开始

前置条件见 [`docs/构建说明.md`](./docs/构建说明.md#前置条件)。

```bash
npm install
npm run tauri:dev
```

常用命令：

| 命令 | 说明 |
|------|------|
| `npm run tauri:dev` | 开发模式 |
| `npm run tauri:build` | 生成 NSIS / MSI 安装包 |
| `npm run check:config` | 校验 `tauri.conf.json`（秒级，编译前抓配置错误） |
| `npm run typecheck` | TypeScript 类型检查 |
| `npm run lint` | ESLint |
| `npm run test` | 前端单元测试（Vitest） |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 后端单元 + 集成测试 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` | Rust lint |
| `cargo fmt --manifest-path src-tauri/Cargo.toml` | Rust 格式化 |

---

## 快捷键

| 键 | 作用 |
|----|------|
| `Ctrl` + `F` | 聚焦搜索框 |
| `↑` `↓` | 移动选中行 |
| `Home` `End` | 跳到首行 / 末行 |
| `PageUp` `PageDown` | 翻页 |
| `Enter` | 打开详情面板 |
| `Delete` | 发起卸载（仍会弹确认框） |
| `Esc` | 关闭最上层浮层 |
| `Alt` + `S` | 打开设置 |

---

## 安全设计

本工具会删除文件与注册表键，因此防护是设计的一部分：

- **命令只接受 `id`**：前端无法提交任意路径。后端持有扫描缓存，卸载 / 残留 / 打开目录都用 `id` 反查真实路径。
- **不猜测静默开关**：只在注册表声明了 `QuietUninstallString` 时使用静默卸载；MSI 之外的程序即使勾选静默也会回退到交互式，并在结果中说明。避免「静默失败但显示成功」。
- **删除路径白名单**：仅允许删除 `C:\Program Files`、`C:\Program Files (x86)`、`C:\ProgramData` 目录**之内**的文件；命中 `Windows` / `Users` / `ProgramData` 等黑名单段或属于联接/符号链接的路径一律拒绝。
- **注册表写入单一入口**：所有删除都经过 `registry::writer::delete_subkey_with_audit`，只允许 `...\CurrentVersion\Uninstall\{子键}`，且逐条写审计日志。
- **展示实际命令**：确认弹窗显示的是**解析后**的可执行文件与参数，而非原始字符串。
- **默认隐藏系统组件**；显示时标注「系统组件」并在确认框给出红色警告。
- **最小 Tauri 权限**：不授予通用 shell 执行能力，CSP 为严格白名单。

---

## 项目结构

```
├── src/                    前端
│   ├── components/         UI 组件
│   ├── stores/             Zustand 状态
│   ├── lib/                IPC 封装与纯函数（可单测）
│   └── i18n/               字典与翻译器
├── src-tauri/
│   ├── src/
│   │   ├── registry/       注册表读取 / 去重 / 受审计的写入
│   │   ├── uninstaller/    命令解析（重度单测）+ 进程执行
│   │   ├── residue.rs      残留检测与清理
│   │   └── commands/       Tauri 命令层
│   ├── tests/              集成测试
│   └── icons/              图标
├── docs/                   技术文档 / 用户手册 / 构建说明
└── scripts/                图标生成等工具脚本
```

---

## 许可

MIT
