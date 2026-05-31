# CyberWeaver MCP Server 工程方案

> 版本：v1.0 | 日期：2026-05-26 | 状态：待评审

---

## 1. 背景与动机

CyberWeaver 本质是一个**有结构的数字取证知识库 + 图分析引擎**，人的主要操作是导入、审阅、连线确认，而大量关联推理、IOC 富化、批量查询、报告生成更适合交给 AI Agent 做。

MCP (Model Context Protocol) 是 Anthropic 发布的开放协议，已成为 AI Agent 与工具交互的事实标准。Claude Code、Codex CLI、Gemini CLI 均已原生支持 MCP。

**核心判断**：CyberWeaver 不是人的主要工作界面，而是 Agent 的「手和眼」——MCP 让 Agent 能直接操作取证知识库，人通过 tldraw 画布做最终决策。

---

## 2. 协议选型

### 2.1 Transport: stdio

| 方案 | 优点 | 缺点 | 结论 |
|---|---|---|---|
| **stdio** | MCP 原生传输、所有客户端支持、零网络配置 | 需启动独立进程 | **采用** |
| HTTP (嵌入 Axum) | 复用现有 WebSocket、同一进程 | MCP HTTP 模式较新、部分客户端不支持 | 备选 |

stdio 模式下，MCP client（Claude Code / Codex / Gemini CLI）以子进程方式启动 `cw-mcp`，通过 stdin/stdout 交换 JSON-RPC 消息。

### 2.2 架构

```
┌──────────────────────────────────────────────┐
│  Claude Code / Codex CLI / Gemini CLI        │
│  (MCP Client, 子进程启动 cw-mcp)              │
└────────────┬─────────────────────────────────┘
             │ stdin/stdout (JSON-RPC 2.0)
             ▼
┌──────────────────────────────────────────────┐
│  cw-mcp (独立 Rust binary)                    │
│                                              │
│  ┌─────────────────────────────────────────┐ │
│  │  MCP Protocol Layer                      │ │
│  │  (rmcp crate: JSON-RPC parse + dispatch) │ │
│  ├─────────────────────────────────────────┤ │
│  │  Tool Implementations                    │ │
│  │  search / traverse / crud / import/export│ │
│  ├─────────────────────────────────────────┤ │
│  │  Shared Library                          │ │
│  │  (复用 CyberWeaver 现有 models/db/graph)  │ │
│  └──────────────┬──────────────────────────┘ │
│                 │                             │
│  ┌──────────────▼──────────────────────────┐ │
│  │  SQLite (WAL) — CyberWeaver 同一数据库    │ │
│  └─────────────────────────────────────────┘ │
└──────────────────────────────────────────────┘
```

### 2.3 MCP 依赖选型

调研现有 Rust MCP SDK：

| Crate | 特点 | 维护状态 |
|---|---|---|
| `rmcp` | 纯 Rust、tokio 原生、支持 stdio/HTTP、类型安全 | 活跃 |
| `mcp-sdk` | 官方参考实现 | 较基础 |
| `poem-mcpserver` | 基于 Poem 框架 | HTTP only |

**选择 `rmcp`**：支持 stdio transport、tokio 兼容、handler 宏简化工具定义、与 CyberWeaver 现有技术栈一致。

---

## 3. Tool 列表

### 3.1 只读查询（Read）

| Tool | 参数 | 返回 | 用途 |
|---|---|---|---|
| `search_nodes` | `investigation_id`, `node_type?`, `keyword?` | `Vec<NodeData>` | 按类型/关键词搜索节点 |
| `get_node` | `node_id` | `NodeData` | 获取节点完整属性 |
| `get_node_neighborhood` | `node_id`, `max_hops?`, `relation_type?` | `{node, neighbors, relations}` | 获取 N 跳邻居子图 |
| `find_path` | `from_id`, `to_id`, `max_hops?` | `TraversalPath` | 两节点间最短路径 |
| `get_graph_summary` | `investigation_id` | `{node_count, type_distribution, relation_count, component_count}` | 图谱概览统计 |
| `list_investigations` | 无 | `Vec<Investigation>` | 列出所有调查案件 |

### 3.2 写入操作（Write）

| Tool | 参数 | 返回 | 用途 |
|---|---|---|---|
| `add_node` | `investigation_id`, `node_type`, `label`, `description?`, `properties?`, `confidence?`, `pos_x?`, `pos_y?` | `NodeData` | 创建节点 |
| `add_relation` | `investigation_id`, `relation_type`, `source_node_id`, `target_node_id`, `label?`, `confidence?` | `RelationData` | 创建关系 |
| `update_node` | `node_id`, `label?`, `description?`, `properties?`, `confidence?` | `NodeData` | 更新节点 |
| `delete_node` | `node_id` | `bool` | 删除节点及关联关系 |
| `delete_relation` | `relation_id` | `bool` | 删除关系 |

### 3.3 导入导出（Import/Export）

| Tool | 参数 | 返回 | 用途 |
|---|---|---|---|
| `import_stix` | `investigation_id`, `stix_json` | `{nodes_imported, relations_imported, errors}` | 导入 STIX 2.1 Bundle |
| `export_stix` | `investigation_id` | `stix_json` | 导出为 STIX 2.1 |
| `import_json_canvas` | `investigation_id`, `canvas_json` | `{nodes_imported, relations_imported, errors}` | 导入 JSON Canvas |
| `export_json_canvas` | `investigation_id` | `canvas_json` | 导出为 JSON Canvas |
| `generate_report` | `investigation_id`, `title?`, `author?` | `html_report` | 生成 HTML 取证报告 |

### 3.4 AI 辅助（需 LLM 配置）

| Tool | 参数 | 返回 | 用途 |
|---|---|---|---|
| `extract_from_text` | `text`, `investigation_id?` | `Vec<ExtractedEntity>` | 从威胁报告文本提取实体 |
| `extract_relations` | `entities`, `text` | `Vec<ExtractedRelation>` | 提取实体间关系 |
| `agent_analyze` | `investigation_id`, `node_ids?` | `AgentPlan` | AI Agent 建议下一步调查操作 |

### 3.5 设计原则

- **只读工具默认安全**：不修改数据库，Agent 可以放心调用
- **写入工具有明确边界**：每个工具只做一件事，返回清晰的结果
- **tool description 面向 LLM 优化**：描述参数含义、枚举值、典型使用场景，帮助模型正确选择工具
- **错误友好**：所有工具返回结构化错误信息而非 panic

---

## 4. 目录结构

```
src-tauri/
├── Cargo.toml                      # [workspace] 包含 cw-mcp
├── src/                            # 现有 Tauri 库代码（不变）
│   └── ...
└── crates/
    └── cw-mcp/
        ├── Cargo.toml
        └── src/
            ├── main.rs             # 入口：启动 MCP server
            ├── server.rs           # MCP Server 初始化 + handler 注册
            ├── tools/
            │   ├── mod.rs
            │   ├── read.rs         # search_nodes, get_node, get_node_neighborhood, find_path, get_graph_summary, list_investigations
            │   ├── write.rs        # add_node, add_relation, update_node, delete_node, delete_relation
            │   ├── import_export.rs # import/export STIX/Canvas + generate_report
            │   └── ai.rs           # extract_from_text, extract_relations, agent_analyze
            ├── db.rs               # 数据库连接（直接打开 SQLite 文件）
            └── error.rs            # MCP 错误类型 → JSON-RPC error
```

### 4.1 Cargo.toml 变更

在 `src-tauri/Cargo.toml` 的 `[workspace]` 中添加 `cw-mcp`：

```toml
[workspace]
members = [
    ".",           # 现有 Tauri 应用
    "crates/cw-mcp",
]
```

`cw-mcp/Cargo.toml` 依赖：

```toml
[package]
name = "cw-mcp"
version = "0.1.0"
edition = "2021"

[dependencies]
# MCP
rmcp = { version = "0.6", features = ["server", "transport-stdio"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 复用 CyberWeaver 库
tauri_app_lib = { path = ".." }    # 复用 models, db, graph, services, error

# 基础设施
sea-orm = { version = "1.1", features = ["sqlx-sqlite", "runtime-tokio-rustls"] }
thiserror = "2"
uuid = { version = "1", features = ["v4"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

**重要**：`cw-mcp` 直接依赖 `tauri_app_lib`（lib crate），复用所有 models、db 层（entities/repositories）、graph engine、services（import/export/report/llm）。不重复写代码。

`tauri_app_lib` 需要将 `lib.rs` 中的 `pub mod` 保持为 pub，确保 `cw-mcp` 可以 `use tauri_app_lib::models::domain::...` 等。

---

## 5. Tool 实现要点

### 5.1 数据库连接

`cw-mcp` 启动时直接从磁盘打开 SQLite 文件（不依赖 Tauri 运行时）：

```rust
// crates/cw-mcp/src/db.rs
use sea_orm::{Database, DatabaseConnection};

pub async fn open_db(db_path: &str) -> Result<DatabaseConnection, AppError> {
    let url = format!("sqlite:{}?mode=rwc", db_path);
    let db = Database::connect(&url).await?;
    // 设置 WAL 模式
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "PRAGMA journal_mode=WAL;".to_string(),
    ))
    .await?;
    Ok(db)
}
```

数据库路径通过 CLI 参数或环境变量 `CW_DB_PATH` 传入，默认：
- macOS: `~/Library/Application Support/com.cyberweaver.app/cyberweaver.db`
- 开发模式：项目根目录下的 `cyberweaver.db`

### 5.2 MCP Server 启动

```rust
// crates/cw-mcp/src/server.rs
use rmcp::{Server, Service, tool, ToolOutput};

pub struct CyberWeaverMcp {
    db: Arc<DatabaseConnection>,
}

impl CyberWeaverMcp {
    pub async fn new(db_path: &str) -> Result<Self, AppError> {
        let db = Arc::new(open_db(db_path).await?);
        Ok(Self { db })
    }
}

#[rmcp::tool]
impl CyberWeaverMcp {
    #[tool(description = "搜索取证节点。可按节点类型和关键词过滤...")]
    async fn search_nodes(
        &self,
        investigation_id: String,
        node_type: Option<String>,
        keyword: Option<String>,
    ) -> Result<Vec<NodeData>, AppError> {
        // 复用 NodeRepo
    }
    // ...其余 tools
}
```

### 5.3 关键 Tool 实现

#### `get_node_neighborhood` — 最常用的图查询

```
输入: node_id, max_hops (默认 2), relation_type (可选)
流程:
  1. NodeRepo::find_by_id(node_id) → 获取中心节点
  2. 加载整个 investigation 的图到 AdjacencyGraph
  3. bfs_paths(graph, node_id, max_hops, relation_type) → N 跳路径
  4. 收集路径中出现的所有节点 + 关系
  5. 返回子图 JSON
输出: { center_node, neighbors: [...], relations: [...] }
```

#### `agent_analyze` — Agent 推理

```
输入: investigation_id, node_ids (可选，不传则用全图)
流程:
  1. 从 DB 加载选中节点（或全图摘要）
  2. 序列化为 LLM prompt
  3. ForensicsAgent::analyze() → AgentPlan
  4. 返回 reasoning + 建议 actions
输出: { reasoning, actions: [{ action, params }] }
注意: 只返回建议，不自动执行。Agent 拿到建议后可调用 add_node/add_relation 执行。
```

---

## 6. MCP 配置示例

### Claude Code (`claude.json` 或 `settings.json`)

```json
{
  "mcpServers": {
    "cyberweaver": {
      "command": "cw-mcp",
      "args": ["--db-path", "/path/to/cyberweaver.db"],
      "description": "CyberWeaver 数字取证调查工作台 - 图谱搜索、STIX 导入导出、报告生成"
    }
  }
}
```

### Codex CLI (`~/.codex/config.toml`)

```toml
[mcp_servers.cyberweaver]
command = "cw-mcp"
args = ["--db-path", "/path/to/cyberweaver.db"]
```

### 开发模式（直接 cargo run）

```json
{
  "mcpServers": {
    "cyberweaver": {
      "command": "cargo",
      "args": ["run", "--bin", "cw-mcp", "--", "--db-path", "./cyberweaver.db"]
    }
  }
}
```

---

## 7. 实施步骤

| 步骤 | 内容 | 预估 |
|---|---|---|
| 1 | 调整 `Cargo.toml` workspace，创建 `crates/cw-mcp` 骨架 | 0.5h |
| 2 | 实现 `cw-mcp/src/db.rs`（数据库连接 + CLI 参数解析） | 0.5h |
| 3 | 实现 `cw-mcp/src/server.rs`（MCP Server 初始化） | 1h |
| 4 | 实现 `tools/read.rs`（6 个只读工具） | 2h |
| 5 | 实现 `tools/write.rs`（5 个写入工具） | 1.5h |
| 6 | 实现 `tools/import_export.rs`（5 个导入导出工具） | 1.5h |
| 7 | 实现 `tools/ai.rs`（3 个 AI 工具，条件可用） | 1h |
| 8 | 集成测试 + Claude Code 实测 | 1h |
| **合计** | | **9h** |

---

## 8. 风险与缓解

| 风险 | 缓解 |
|---|---|
| `tauri_app_lib` 依赖 Tauri 特定类型（如 `State`）无法在纯 stdio 环境编译 | 将可复用代码（models, db, graph, services）提取为独立 `cyberweaver-core` crate，`tauri_app_lib` 和 `cw-mcp` 都依赖它；或者在 `cw-mcp` 中 conditionally 绕过 Tauri 依赖 |
| `rmcp` API 不稳定 | 锁定版本，MCP 协议层薄封装便于替换 |
| 两个进程同时写 SQLite 导致锁冲突 | SQLite WAL 模式支持多读单写，MCP 写入量极小（人工驱动的 Agent 操作），概率低；必要时加 busy_timeout |

> **关于风险 1 的详细分析**：当前 `tauri_app_lib` 的核心依赖中，真正依赖 Tauri 运行时（`State`, `Manager`）的是 `commands/` 目录和 `lib.rs` 的 `run()` 函数。`models/`, `db/`, `graph/`, `services/`, `error.rs` 是纯逻辑，不依赖 Tauri。因此 `cw-mcp` 可以直接 `use tauri_app_lib::models::domain` 等，无需拆分 crate。**唯一需确认**的是 `Cargo.toml` 中 `tauri` 依赖是否会导致编译时链接问题——`tauri` crate 在非 Tauri 环境下可以编译（它是一个库依赖），只是不能调用需要 `AppHandle` 的函数。验证方式：`cargo build --bin cw-mcp` 看是否报链接错误。

---

## 9. 后续演进

- **双向同步**：MCP 写入后通过 WebSocket 通知 tldraw 前端实时更新
- **Resources**：将 Investigation 暴露为 `investigation://<id>` 资源，Agent 可订阅
- **Prompts**：预置常用取证分析 prompt 模板（"分析这个 APT 组织的攻击链"、"找出所有未连接的孤立 IOC"）
- **MCP HTTP 模式**：未来如需远程 Agent 访问，可在 Axum 上加 MCP HTTP endpoint

---

> 撰写人：Claude Code | 2026-05-26
