# CyberWeaver — 数字取证调查工作台

![Tauri v2](https://img.shields.io/badge/Tauri-v2-24C8DB?style=flat-square&logo=tauri&logoColor=white)
![React 19](https://img.shields.io/badge/React-19-149ECA?style=flat-square&logo=react&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-1.80+-000000?style=flat-square&logo=rust&logoColor=white)
![SQLite](https://img.shields.io/badge/SQLite-WAL-003B57?style=flat-square&logo=sqlite&logoColor=white)

CyberWeaver 是一个面向数字取证与威胁调查场景的桌面工作台，基于 Tauri v2 + React 19 + tldraw 无限画布构建。核心目标是把分散的 IOC、进程、恶意软件、攻击技术与资产信息，组织成可追踪、可分析、可导入导出的调查图谱。

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

## 快速开始

### 前置要求

- Node.js 20+
- Rust 1.80+
- pnpm

以及 Tauri v2 的[系统依赖](https://v2.tauri.app/start/prerequisites/)。

### 安装与开发

```bash
git clone https://github.com/loopy898/CyberWeaver
cd CyberWeaver
pnpm install
pnpm tauri dev
```

### 构建生产包

```bash
pnpm tauri build
```

---

## 技术栈

| 层级 | 技术 |
|---|---|
| 桌面框架 | Tauri v2 |
| 前端渲染 | React 19 + Vite 7 |
| 可视化画布 | tldraw 4.x |
| 状态管理 | Zustand |
| 后端语言 | Rust（稳定版） |
| 异步运行时 | Tokio |
| Web 服务 | Axum 0.8（REST + WebSocket） |
| 数据库 | SQLite（WAL 模式） |
| ORM | SeaORM 1.x |
| LLM 接入 | reqwest → OpenAI 兼容 Chat Completions API |
| 序列化 | serde + serde_json |
| 错误处理 | thiserror |
| 数据交换 | JSON Canvas · STIX 2.1 · Attack Flow · HTML Report |

---

## 项目结构

```
CyberWeaver/
├── src/                          # React 前端
│   ├── components/
│   │   ├── canvas/               # 调查画布、自定义形状、工具
│   │   └── panels/               # AI 面板、属性面板、导入导出、遍历分析
│   ├── hooks/                    # usePersistence / useWebSocket / useLLM / useImportExport
│   ├── lib/                      # 常量、形状映射
│   ├── stores/                   # Zustand 状态（当前调查、选中节点等）
│   └── types/                    # 领域类型定义
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── ai/                   # Agent 推理与动作定义
│   │   ├── commands/             # Tauri IPC 命令入口
│   │   ├── db/                   # 数据库连接、实体、仓储、迁移
│   │   ├── graph/                # 内存邻接表、图遍历引擎
│   │   ├── models/               # 领域模型 + STIX/CANVAS/AFB 格式映射
│   │   ├── services/
│   │   │   ├── import/           # 外部格式导入
│   │   │   ├── llm/              # LLM 客户端、提取器、提示词
│   │   │   └── report/           # HTML 报告生成
│   │   ├── ws/                   # WebSocket 协议与推送
│   │   ├── lib.rs                # 应用初始化与命令注册
│   │   ├── main.rs               # 入口
│   │   └── state.rs              # 全局状态
│   ├── tests/                    # 集成测试
│   └── Cargo.toml
├── docs/plans/                   # 实施计划文档
└── README.md
```

---

## LLM 配置

在 AI 面板中填写以下参数：

- **API Base** — 模型服务地址（如 `https://api.openai.com` 或 `http://127.0.0.1:11434`）
- **API Key** — 访问密钥；本地服务可留空
- **Model** — 模型名（如 `gpt-4o`、`llama3.1`）

接口会拼接为 `<API Base>/v1/chat/completions`，认证头为 `Authorization: Bearer <API Key>`。

---

## 测试

```bash
# Rust 单元测试与集成测试
cd src-tauri && cargo test

# 前端类型检查
npx tsc --noEmit
```

---

## 许可证

MIT License — 详见 [LICENSE](./LICENSE) 文件。
