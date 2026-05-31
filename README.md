# CyberWeaver — 数字取证调查工作台






CyberWeaver 是一个面向数字取证与威胁调查场景的桌面工作台，基于 Tauri v2 + React 19 + tldraw 无限画布构建。核心目标是把分散的 IOC、进程、恶意软件、攻击技术与资产信息，组织成可追踪、可分析、可导入导出的调查图谱。

它的目标不是“再造一个白板”，而是把零散线索（IP、进程、文件、事件）沉淀成结构化数据，并在无限画布里可视化串联。

---

## 功能概览

### 自定义领域节点
内置 8 种调查节点类型，覆盖常见取证与威胁情报对象：

| 类型 | 说明 |
|---|---|
| `IpAddress` | IP 地址（IPv4/IPv6），含地理位置、ASN、ISP、信誉 |
| `Domain` | 域名，含注册商、创建时间 |
| `FileHash` | 文件哈希（MD5/SHA1/SHA256），含文件名、大小、类型 |
| `Process` | 进程，含 PID、命令行、父进程、用户 |
| `Malware` | 恶意软件家族，含别名、类型、首次发现时间 |
| `Ttp` | MITRE ATT&CK 攻击技术，含战术、平台、数据源 |
| `ThreatActor` | 威胁组织/个人，含别名、动机、能力等级、目标行业 |
| `Asset` | 资产/主机，含操作系统、IP 列表、所有者、关键等级 |

### 图关系建模
7 种有向关系边，用于表达攻击链、主机行为、样本归属和基础设施映射：

`ConnectsTo` · `ResolvesTo` · `Creates` · `BelongsTo` · `Uses` · `Targets` · `Contains`

### AI 辅助提取
粘贴威胁报告 → LLM 提取实体与关系 → 人工确认 → 自动生成图谱。
- 支持 OpenAI 及 Ollama 等兼容 `/v1/chat/completions` 接口的模型
- 实体提取 + 关系推理双阶段流水线
- Agent 模式：选中节点 → AI 建议关联 → 审批后自动连线

### 多格式导入导出
| 方向 | 格式 |
|---|---|
| 导入 | JSON Canvas · STIX 2.1 · Attack Flow |
| 导出 | JSON Canvas · STIX 2.1 · Attack Flow · HTML Report |

可与 Obsidian、OpenCTI、MISP 等外部工具互通。

### 持久化与实时推送
- 本地 SQLite（WAL 模式）存储全量调查数据
- SeaORM 仓储层统一管理读写
- Axum WebSocket 广播状态变更，支持实时同步

---

## LLM 配置
在 AI 面板中填写以下参数：
- **API Base** — 模型服务地址（如 `https://api.openai.com` 或 `http://127.0.0.1:11434`）
- **API Key** — 访问密钥；本地服务可留空
- **Model** — 模型名（如 `gpt-4o`、`llama3.1`）

接口会拼接为 `<API Base>/v1/chat/completions`，认证头为 `Authorization: Bearer <API Key>`。

---

## 核心能力
- 结构化线索模型（`geo` / `text` / `note`）
- 画布加载安全校验，避免历史脏数据触发 `ValidationError`
- 画布与 SQLite 的稳定双向同步
  - 增量 upsert
  - 删除同步
  - 去抖合并写入
  - 后端事务处理
- SQLite schema 自动迁移（兼容旧表结构）
- 浏览器模式持久化回退（便于 Web 调试与 e2e）

## 技术栈
- Frontend: React 19, TypeScript, tldraw, Vite
- Desktop shell: Tauri 2
- Backend: Rust, SeaORM, SQLite
- Testing: Vitest, Playwright

## 架构概览
```text
src/
  domain/
    clueNode.ts        # 线索领域模型与校验
    shapeMapper.ts     # tldraw shape <-> 线索实体映射
    storeChanges.ts    # store diff -> 持久化增量
  hooks/
    useCanvasSync.ts   # 画布同步控制器（加载、监听、写回）
  infrastructure/
    nodeGateway.ts     # tauri 命令网关 + 浏览器回退存储

src-tauri/src/
  lib.rs               # tauri 命令、数据库初始化、迁移、同步写入
  main.rs              # tauri 入口
