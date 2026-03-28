# CyberWeaver🕷️

**CyberWeaver** 是一款基于 Tauri + React + Tldraw 开发的开源数字取证工作台。它通过无限画布将零散的攻击线索（IP、进程、文件）转化为直观的拓扑图谱，旨在为安全分析师提供高效的“线索编织”体验。

---

## 🚩 开发进度 (Roadmap)

### 第一阶段：环境搭建与基础可视化 (已完成)
- [x] 基于 Tauri 的桌面客户端骨架搭建 **(已完成)**
- [x] Tldraw 无限画布引擎集成 **(已完成)**
- [x] 基础 UI 界面与交互逻辑配置 **(已完成)**

### 第二阶段：核心数据层与持久化 (进行中)
- [x] 后端 SQLite 数据库初始化与 SeaORM 映射 **(已完成)**
- [x] 定义结构化线索实体（矩形、文本、便签） **(已完成)**
- [ ] 修复线索加载时的 Tldraw 属性校验报错 (ValidationError)
- [ ] 实现画布数据与 SQLite 的双向稳定同步

### 第三阶段：同构旁路与可视化增强 (待启动)
- [x] 集成 Axum 服务，建立 WebSocket 实时双向通信
- [x] 实现后端对前端画布的“反向操作”（AI 绘图预览）
- [x] 引入图形自动布局算法 (Elkjs) 与 LOD 视图切换

### 第四阶段：AI 智能体与自动化取证 (待启动)
- [x] Python 分析脚本/Agent 插件化接口（`scan_port` 示例）
- [x] 攻击路径自动推理占位链路（ToolResult + AgentToken 流）
- [ ] 取证报告一键导出

---

## 🛡️ 取证规范说明
为了保证数据的结构化分析能力，本项目对操作进行了严格区分：
1. **正式线索**：仅限使用【矩形】、【文字】和【便签】。这些数据会被实时存入数据库。
2. **临时标记**：【涂鸦】和【箭头】等目前仅作为视觉辅助，不会被持久化，请知悉。

## 🛠️ 快速开始
1. **安装依赖**: `npm install`
2. **启动开发模式**: `npm run tauri dev`
3. **数据重置**: 若遇到 ValidationError 报错，请手动删除 `src-tauri/cyberweaver.db` 并重启。

---

## 🔌 新增能力（2026-03-27）

### 后端核心数据层
- SQLite 三表结构：`nodes`、`edges`、`components`
- 内存 `GraphManager` 热缓存 + 异步持久化
- REST API:
  - `GET /graph`
  - `POST /node`
  - `POST /edge`

### WebSocket 协议
- 命令（Command）
  - `debug_create_node`
  - `tool_execution`
- 事件（Event）
  - `graph_update`
  - `tool_result`
  - `agent_token`

### 前端可视化增强
- 后端图谱增量同步到画布
- Elkjs 自动布局
- Zoom LOD：缩小时渲染圆点（`geo`），放大后渲染卡片（`note`）
- `ScanPort` 按钮：对选中节点发起自动化指令
- Agent 思考气泡（token 流）
- 全新控制台 UI：状态徽标、实时节点/边计数、快照刷新、移动端适配

### 插件系统
- `plugins/sdk.py`：`@register_tool` 注册机制
- `plugins/scan_port.py`：端口扫描示例插件（模拟输出）

---

## ✅ 自测命令

前端:
- `node --test src/ws.test.ts`
- `npm run build`

后端:
- `cd src-tauri`
- `cargo test`
- `cargo check`

手动验证链路:
1. 启动应用后点击「推送测试节点」确认 `graph_update` 到达画布。
2. 选中一个节点点击 `ScanPort`，确认状态栏显示 `tool_result`。
3. 观察画布中新增扫描结果节点与关联边，并出现 Agent 思考气泡文本流。
