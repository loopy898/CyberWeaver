# CyberWeaver 插件框架工程方案

> 版本：v1.0 | 日期：2026-05-26 | 状态：待评审

---

## 1. 背景与动机

当前 Agent 的 `QueryExternal` action（VirusTotal/Whois/Shodan）只有定义没有实现。同时第三方无法将自己的取证工具（YARA 扫描器、GeoIP 库、威胁情报 API 等）接入 CyberWeaver。

**目标**：定义 `InvestigationTool` trait + C ABI 边界 + 动态库加载，让第三方能用 Rust 实现工具，编译为 `.so/.dylib`，CyberWeaver 启动时自动发现并注册，Agent 可以调用。

---

## 2. 整体架构

```
┌─────────────────────────────────────────────────────────┐
│  CyberWeaver Agent                                       │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Tool Registry                                   │    │
│  │  ┌──────────┐  ┌──────────┐  ┌───────────────┐  │    │
│  │  │ 内置工具  │  │ 动态库A   │  │ 动态库B        │  │    │
│  │  │ VirusTotal│  │ yara.so  │  │ geoip.dylib  │  │    │
│  │  │ Whois     │  │          │  │               │  │    │
│  │  │ Shodan    │  │          │  │               │  │    │
│  │  └──────────┘  └──────────┘  └───────────────┘  │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  Agent 流程：                                            │
│  1. 查询 Registry 获取所有 ToolManifest                  │
│  2. 将 manifest 列表注入 LLM prompt                      │
│  3. LLM 返回 UseTool { tool_name, params }               │
│  4. 用户审批后，Registry 执行工具                         │
│  5. ToolOutput 的 new_nodes/relations 写入图谱           │
└─────────────────────────────────────────────────────────┘
```

---

## 3. 核心数据结构

### 3.1 工具清单（Agent 看到的"说明书"）

```rust
/// 工具的元数据描述 — Agent 根据这些信息决定调用哪个工具
pub struct ToolManifest {
    pub name: String,                    // 唯一标识: "virustotal_ip_lookup"
    pub display_name: String,            // 展示名: "VirusTotal IP 查询"
    pub description: String,             // 详细描述（Agent 读这个）
    pub version: String,                 // "1.0.0"
    pub author: String,                  // 作者
    pub parameters: Vec<ToolParameter>,  // 输入参数 schema
    pub input_types: Vec<NodeType>,      // 接受什么类型的节点（Agent 过滤用）
    pub output_types: Vec<NodeType>,     // 可能产出什么类型的新节点
}

pub struct ToolParameter {
    pub name: String,
    pub parameter_type: ParameterType,   // String / Integer / Float / Boolean
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
}

pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
}
```

### 3.2 工具输入/输出

```rust
/// 工具输入 — 可以关联到一个已有节点，也可以传自由参数
pub struct ToolInput {
    pub node_id: Option<String>,         // 可选：关联节点 ID
    pub params: serde_json::Value,       // 自由结构（JSON Object）
}

/// 工具输出 — Agent 拿到结果后决定写入哪些节点和关系
pub struct ToolOutput {
    pub new_nodes: Vec<DiscoveredNode>,     // 新发现的实体
    pub new_relations: Vec<DiscoveredRelation>, // 新发现的关系
    pub enriched_properties: serde_json::Value, // 对输入节点的属性补充
    pub text_summary: String,               // 给人/Agent 看的摘要
}

pub struct DiscoveredNode {
    pub node_type: String,    // snake_case: "ip_address", "domain", etc.
    pub label: String,
    pub description: String,
    pub properties: serde_json::Value,
    pub confidence: f32,
}

pub struct DiscoveredRelation {
    pub source_label: String,     // 用 label 引用节点（Agent 友好）
    pub target_label: String,
    pub relation_type: String,    // snake_case: "connects_to", etc.
    pub label: String,
    pub confidence: f32,
}
```

### 3.3 Agent 新 Action

```rust
pub enum AgentAction {
    AddNode { ... },
    AddRelation { ... },
    QueryExternal { ... },
    /// 新增：调用注册的工具
    UseTool {
        tool_name: String,                    // manifest.name
        params: serde_json::Value,            // 按 parameters schema 填充
        auto_merge: bool,                     // true = 自动写入图谱，false = 只返回结果
    },
}
```

---

## 4. C ABI 边界

Rust trait 不是 ABI 稳定的（不同编译器版本会崩），所以动态库边界必须用 C ABI + JSON 序列化。

### 4.1 插件导出函数

每个 `.so/.dylib` 必须导出以下 C 函数：

```c
/// 版本检查 — 插件 SDK 版本必须匹配
uint32_t cw_plugin_version(void);

/// 创建插件实例，返回不透明句柄
/// 返回 NULL 表示创建失败
void* cw_plugin_new(void);

/// 获取工具清单（JSON 字符串）
/// 调用方负责用 cw_string_free 释放
const char* cw_plugin_manifest(void* handle);

/// 执行工具（异步阻塞调用）
/// input_json: JSON 序列化的 ToolInput
/// 返回 JSON 序列化的 ToolOutput
/// 调用方负责用 cw_string_free 释放
const char* cw_plugin_execute(void* handle, const char* input_json);

/// 销毁插件实例
void cw_plugin_destroy(void* handle);

/// 释放字符串
void cw_string_free(const char* s);
```

### 4.2 插件 SDK 宏

第三方只需写 trait 实现 + 一行宏：

```rust
use cw_plugin_sdk::{InvestigationTool, ToolInput, ToolOutput, ToolManifest, export_plugin};

struct MyScanner { /* 内部状态 */ }

impl InvestigationTool for MyScanner {
    fn manifest(&self) -> ToolManifest { ... }
    fn execute(&self, input: ToolInput) -> Result<ToolOutput, String> { ... }
}

// 一行宏：生成所有 C ABI 导出函数
export_plugin!(MyScanner);
```

`export_plugin!` 宏内部生成：

```rust
#[no_mangle] pub extern "C" fn cw_plugin_version() -> u32 { 1 }
#[no_mangle] pub extern "C" fn cw_plugin_new() -> *mut MyScanner { ... }
#[no_mangle] pub extern "C" fn cw_plugin_manifest(handle: *mut MyScanner) -> *const c_char { ... }
#[no_mangle] pub extern "C" fn cw_plugin_execute(handle: *mut MyScanner, input: *const c_char) -> *const c_char { ... }
#[no_mangle] pub extern "C" fn cw_plugin_destroy(handle: *mut MyScanner) { ... }
#[no_mangle] pub extern "C" fn cw_string_free(s: *const c_char) { ... }
```

---

## 5. 目录结构

### 5.1 新增 Crate

```
src-tauri/
├── crates/
│   ├── cw-mcp/                    # 已有
│   ├── cw-plugin-sdk/             # 新增：插件 SDK
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs             # InvestigationTool trait + 数据结构
│   │       ├── types.rs           # ToolManifest, ToolInput, ToolOutput, etc.
│   │       └── export_macro.rs    # export_plugin! 宏
│   └── cw-plugins-builtin/        # 新增：内置插件
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── virustotal.rs      # VirusTotal IP/Hash 查询
│           ├── whois.rs           # WHOIS 域名查询
│           └── shodan.rs          # Shodan IP 扫描
```

### 5.2 主应用变更

```
src-tauri/src/
├── plugins/                       # 新增：插件加载框架
│   ├── mod.rs
│   ├── loader.rs                  # libloading 动态库加载
│   ├── registry.rs                # ToolRegistry — 管理所有工具
│   └── executor.rs                # 工具执行 + 结果合并到图谱
├── ai/
│   ├── actions.rs                 # 新增 UseTool variant
│   ├── agent.rs                   # Agent 感知 Registry
│   └── mod.rs
```

---

## 6. 插件加载流程

```rust
// plugins/loader.rs

use libloading::{Library, Symbol};

pub struct LoadedPlugin {
    _library: Library,            // 持有 so 句柄，drop 时自动卸载
    handle: *mut c_void,          // 插件实例句柄
    manifest: ToolManifest,       // 缓存的 manifest
}

pub fn load_plugin(path: &Path) -> Result<LoadedPlugin, PluginError> {
    unsafe {
        let lib = Library::new(path)?;

        // 版本检查
        let version_fn: Symbol<unsafe extern "C" fn() -> u32> = lib.get(b"cw_plugin_version")?;
        let version = version_fn();
        if version != SDK_VERSION {
            return Err(PluginError::VersionMismatch { expected: SDK_VERSION, got: version });
        }

        // 创建实例
        let new_fn: Symbol<unsafe extern "C" fn() -> *mut c_void> = lib.get(b"cw_plugin_new")?;
        let handle = new_fn();
        if handle.is_null() {
            return Err(PluginError::InitFailed);
        }

        // 获取 manifest
        let manifest_fn: Symbol<unsafe extern "C" fn(*mut c_void) -> *const c_char> =
            lib.get(b"cw_plugin_manifest")?;
        let manifest_cstr = manifest_fn(handle);
        let manifest_json = CStr::from_ptr(manifest_cstr).to_str()?.to_string();
        let manifest: ToolManifest = serde_json::from_str(&manifest_json)?;

        Ok(LoadedPlugin {
            _library: lib,
            handle,
            manifest,
        })
    }
}

/// 扫描目录加载所有插件
pub fn discover_plugins(dir: &Path) -> Vec<LoadedPlugin> {
    let mut plugins = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_plugin_file(&path) {
                match load_plugin(&path) {
                    Ok(plugin) => {
                        tracing::info!("loaded plugin: {} from {}", plugin.manifest.name, path.display());
                        plugins.push(plugin);
                    }
                    Err(err) => {
                        tracing::warn!("failed to load plugin {}: {err}", path.display());
                    }
                }
            }
        }
    }
    plugins
}
```

---

## 7. ToolRegistry

```rust
// plugins/registry.rs

pub struct ToolRegistry {
    builtin_tools: Vec<Box<dyn InvestigationTool>>,
    dynamic_plugins: Vec<LoadedPlugin>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            builtin_tools: Vec::new(),
            dynamic_plugins: Vec::new(),
        }
    }

    /// 注册内置工具
    pub fn register_builtin(&mut self, tool: Box<dyn InvestigationTool>) {
        self.builtin_tools.push(tool);
    }

    /// 加载动态库插件
    pub fn load_plugins_from(&mut self, dir: &Path) {
        self.dynamic_plugins.extend(discover_plugins(dir));
    }

    /// 获取所有工具的 manifest
    pub fn all_manifests(&self) -> Vec<ToolManifest> {
        let mut manifests: Vec<ToolManifest> = self.builtin_tools
            .iter()
            .map(|t| t.manifest())
            .collect();
        for plugin in &self.dynamic_plugins {
            manifests.push(plugin.manifest.clone());
        }
        manifests
    }

    /// 按名称执行工具
    pub async fn execute(
        &self,
        tool_name: &str,
        input: ToolInput,
    ) -> Result<ToolOutput, PluginError> {
        // 先查内置
        for tool in &self.builtin_tools {
            if tool.manifest().name == tool_name {
                return tool.execute(input).await;
            }
        }
        // 再查动态库
        for plugin in &self.dynamic_plugins {
            if plugin.manifest.name == tool_name {
                return plugin.execute_sync(input);  // FFI 调用，内部可能阻塞
            }
        }
        Err(PluginError::NotFound(tool_name.to_string()))
    }
}
```

---

## 8. Agent 集成

### 8.1 增强 Prompt

Agent prompt 中加入工具清单：

```
Available investigation tools:
- virustotal_ip_lookup(ip_address) → discovers: domains, malware
- whois_domain(domain) → discovers: ip_address, domain (registrar)
- shodan_scan(ip_address) → discovers: process, asset, domain
- ...

When analyzing the graph, you may use UseTool actions to query external
data sources. Tool outputs are automatically merged into the investigation.
```

### 8.2 执行流程

```
Agent.analyze()
    ↓
LLM 返回 AgentPlan { actions: [..., UseTool { tool_name: "virustotal_ip_lookup", params: {...} }] }
    ↓
用户审批 UseTool action
    ↓
Registry.execute("virustotal_ip_lookup", input)
    ↓
ToolOutput { new_nodes: [...], new_relations: [...], text_summary: "..." }
    ↓
executor.merge_output(db, investigation_id, output)
    ↓
新节点/关系写入 SQLite，WebSocket 广播通知前端
```

---

## 9. 第三方插件开发示例

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib"]    # 关键：编译为动态库

[dependencies]
cw-plugin-sdk = { path = "/path/to/cw-plugin-sdk" }
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
serde_json = "1"
```

```rust
// src/lib.rs
use cw_plugin_sdk::*;

struct GeoIpTool;

impl InvestigationTool for GeoIpTool {
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            name: "geoip_lookup".to_string(),
            display_name: "GeoIP 地理位置查询".to_string(),
            description: "查询 IP 地址的地理位置、ASN、ISP 信息".to_string(),
            version: "1.0.0".to_string(),
            author: "third-party-dev".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "ip_address".to_string(),
                    parameter_type: ParameterType::String,
                    description: "要查询的 IP 地址".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            input_types: vec!["ip_address".to_string()],
            output_types: vec!["ip_address".to_string(), "asset".to_string()],
        }
    }

    fn execute(&self, input: ToolInput) -> Result<ToolOutput, String> {
        let ip = input.params["ip_address"]
            .as_str()
            .ok_or("missing ip_address parameter")?;

        // 调用 ipinfo.io API（举例）
        let resp = reqwest::blocking::get(format!("https://ipinfo.io/{ip}/json"))
            .map_err(|e| e.to_string())?;
        let data: serde_json::Value = resp.json().map_err(|e| e.to_string())?;

        Ok(ToolOutput {
            new_nodes: vec![
                DiscoveredNode {
                    node_type: "asset".to_string(),
                    label: data["org"].as_str().unwrap_or("").to_string(),
                    description: format!("ISP: {}", data["org"].as_str().unwrap_or("unknown")),
                    properties: serde_json::json!({
                        "type": "asset",
                        "hostname": data["hostname"].as_str().unwrap_or("")
                    }),
                    confidence: 0.8,
                },
            ],
            new_relations: vec![
                DiscoveredRelation {
                    source_label: ip.to_string(),
                    target_label: data["org"].as_str().unwrap_or("").to_string(),
                    relation_type: "belongs_to".to_string(),
                    label: "ISP".to_string(),
                    confidence: 0.8,
                },
            ],
            enriched_properties: serde_json::json!({
                "geo_location": data["loc"].as_str().unwrap_or(""),
                "isp": data["org"].as_str().unwrap_or(""),
            }),
            text_summary: format!(
                "IP {} → 位置: {}, ISP: {}",
                ip,
                data["city"].as_str().unwrap_or("unknown"),
                data["org"].as_str().unwrap_or("unknown")
            ),
        })
    }
}

export_plugin!(GeoIpTool);
```

编译：

```bash
cargo build --release
# 产物：target/release/libgeoip.so (Linux) 或 libgeoip.dylib (macOS)
```

复制到 CyberWeaver 插件目录：

```bash
cp target/release/libgeoip.dylib ~/.cyberweaver/plugins/
```

启动 CyberWeaver 时自动发现并加载。

---

## 10. 实施步骤

| 步骤 | 内容 | 预估 |
|---|---|---|
| 1 | 创建 `cw-plugin-sdk` crate：trait + 数据结构 + export_plugin! 宏 | 3h |
| 2 | 实现 `plugins/loader.rs`：libloading 动态库加载 + 版本检查 | 2h |
| 3 | 实现 `plugins/registry.rs`：ToolRegistry 统一管理 | 1h |
| 4 | 实现 `plugins/executor.rs`：工具输出合并到图谱（写入 DB + WS 广播） | 2h |
| 5 | 增强 `ai/actions.rs`：新增 UseTool variant | 0.5h |
| 6 | 增强 `ai/agent.rs`：Agent prompt 注入工具清单 + 处理 UseTool 结果 | 2h |
| 7 | 实现内置工具：VirusTotal + Whois + Shodan（至少 2 个） | 3h |
| 8 | 创建示例插件 crate + 文档 | 1h |
| 9 | 测试：加载动态库 → 执行 → 验证图谱写入 | 1.5h |
| **合计** | | **16h** |

---

## 11. 风险与缓解

| 风险 | 缓解 |
|---|---|
| Rust ABI 不稳定导致插件崩溃 | 强制 C ABI + JSON 序列化，不传递 Rust 类型跨 FFI |
| 插件 panic 导致主进程崩溃 | 在 FFI 边界 catch_unwind，插件 panic 只影响该次调用 |
| 插件阻塞主线程 | FFI 调用在 tokio::task::spawn_blocking 中执行 |
| 插件版本不匹配 | 加载时检查 cw_plugin_version()，不匹配则跳过并告警 |
| 动态库内存泄漏 | LoadedPlugin 的 Drop impl 调用 cw_plugin_destroy |

---

## 12. 后续演进

- **插件市场**：远程插件仓库，一键安装/卸载
- **热加载**：文件监控 plugins/ 目录，运行时加载/卸载
- **权限沙箱**：每个插件声明需要的权限（网络/文件/进程），用户审批
- **WASM 支持**：wasmtime 作为第二后端，支持更安全的隔离

---

> 撰写人：Claude Code | 2026-05-26
