use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::db::entities::node;
use crate::error::AppError;

pub struct NodeRepo<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> NodeRepo<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// 获取调查案件中的所有节点
    pub async fn find_by_investigation(
        &self,
        investigation_id: &str,
    ) -> Result<Vec<node::Model>, AppError> {
        let nodes = node::Entity::find()
            .filter(node::Column::InvestigationId.eq(investigation_id))
            .order_by_asc(node::Column::CreatedAt)
            .all(self.db)
            .await?;
        Ok(nodes)
    }

    /// 按 ID 查找单个节点
    pub async fn find_by_id(&self, id: &str) -> Result<Option<node::Model>, AppError> {
        let node = node::Entity::find_by_id(id).one(self.db).await?;
        Ok(node)
    }

    /// 按类型筛选节点
    pub async fn find_by_type(
        &self,
        investigation_id: &str,
        node_type: &str,
    ) -> Result<Vec<node::Model>, AppError> {
        let nodes = node::Entity::find()
            .filter(node::Column::InvestigationId.eq(investigation_id))
            .filter(node::Column::NodeType.eq(node_type))
            .all(self.db)
            .await?;
        Ok(nodes)
    }

    /// 创建新节点
    pub async fn create(&self, data: CreateNodeData) -> Result<node::Model, AppError> {
        let now = chrono_now();

        let active = node::ActiveModel {
            id: Set(if data.fixed_id.is_empty() {
                Uuid::new_v4().to_string()
            } else {
                data.fixed_id
            }),
            investigation_id: Set(data.investigation_id),
            node_type: Set(data.node_type),
            label: Set(data.label),
            description: Set(data.description),
            confidence: Set(data.confidence),
            properties: Set(data.properties),
            pos_x: Set(data.pos_x),
            pos_y: Set(data.pos_y),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        };

        let model = active.insert(self.db).await?;
        Ok(model)
    }

    /// 更新节点
    pub async fn update(&self, id: &str, data: UpdateNodeData) -> Result<node::Model, AppError> {
        let mut active: node::ActiveModel = node::Entity::find_by_id(id)
            .one(self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("node not found: {id}")))?
            .into();

        if let Some(label) = data.label {
            active.label = Set(label);
        }
        if let Some(description) = data.description {
            active.description = Set(description);
        }
        if let Some(confidence) = data.confidence {
            active.confidence = Set(confidence);
        }
        if let Some(properties) = data.properties {
            active.properties = Set(properties);
        }
        if let Some(pos_x) = data.pos_x {
            active.pos_x = Set(pos_x);
        }
        if let Some(pos_y) = data.pos_y {
            active.pos_y = Set(pos_y);
        }
        active.updated_at = Set(chrono_now());

        let model = active.update(self.db).await?;
        Ok(model)
    }

    /// 删除节点（级联删除关联的关系）
    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        node::Entity::delete_by_id(id).exec(self.db).await?;
        Ok(())
    }

    /// 批量保存（用于画布同步：INSERT OR REPLACE）
    pub async fn upsert_batch(
        &self,
        nodes: Vec<CreateNodeData>,
    ) -> Result<Vec<node::Model>, AppError> {
        let mut results = Vec::with_capacity(nodes.len());
        for data in nodes {
            // 先尝试删除旧记录，再插入
            let _ = node::Entity::delete_by_id(&data.fixed_id)
                .exec(self.db)
                .await;
            // 使用固定 ID（从画布传来的）
            let now = chrono_now();
            let active = node::ActiveModel {
                id: Set(data.fixed_id),
                investigation_id: Set(data.investigation_id),
                node_type: Set(data.node_type),
                label: Set(data.label),
                description: Set(data.description),
                confidence: Set(data.confidence),
                properties: Set(data.properties),
                pos_x: Set(data.pos_x),
                pos_y: Set(data.pos_y),
                created_at: Set(now.clone()),
                updated_at: Set(now),
            };
            results.push(active.insert(self.db).await?);
        }
        Ok(results)
    }
}

pub struct CreateNodeData {
    pub fixed_id: String,
    pub investigation_id: String,
    pub node_type: String,
    pub label: String,
    pub description: String,
    pub confidence: f32,
    pub properties: String,
    pub pos_x: f64,
    pub pos_y: f64,
}

pub struct UpdateNodeData {
    pub label: Option<String>,
    pub description: Option<String>,
    pub confidence: Option<f32>,
    pub properties: Option<String>,
    pub pos_x: Option<f64>,
    pub pos_y: Option<f64>,
}

/// 返回 SQLite 兼容的日期时间字符串，格式 "YYYY-MM-DD HH:MM:SS"
pub fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            let days = secs / 86400;
            let time_secs = secs % 86400;
            let hours = time_secs / 3600;
            let mins = (time_secs % 3600) / 60;
            let secs = time_secs % 60;
            format_iso(days, hours, mins, secs)
        })
        .unwrap_or_else(|_| "1970-01-01 00:00:00".to_string())
}

fn format_iso(days: u64, hours: u64, mins: u64, secs: u64) -> String {
    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 0;
    for (i, &md) in month_days.iter().enumerate() {
        if remaining < md as i64 {
            m = i + 1;
            break;
        }
        remaining -= md as i64;
        m = i + 1;
    }
    let d = remaining + 1;
    format!("{y:04}-{m:02}-{d:02} {hours:02}:{mins:02}:{secs:02}")
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ActiveModelTrait, Database, Set};

    use crate::db::entities::investigation;
    use crate::db::migrations::run_migrations;

    async fn setup_db() -> sea_orm::DatabaseConnection {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite should connect");
        run_migrations(&db)
            .await
            .expect("test migrations should succeed");

        investigation::ActiveModel {
            id: Set("inv-1".to_string()),
            name: Set("Investigation".to_string()),
            description: Set("Test investigation".to_string()),
            created_at: Set(chrono_now()),
            updated_at: Set(chrono_now()),
        }
        .insert(&db)
        .await
        .expect("test investigation should insert");

        db
    }

    #[tokio::test]
    async fn node_repo_crud_roundtrip() {
        let db = setup_db().await;
        let repo = NodeRepo::new(&db);

        let created = repo
            .create(CreateNodeData {
                fixed_id: String::new(),
                investigation_id: "inv-1".to_string(),
                node_type: "ip_address".to_string(),
                label: "10.0.0.1".to_string(),
                description: "Initial node".to_string(),
                confidence: 0.7,
                properties: r#"{"type":"ip_address"}"#.to_string(),
                pos_x: 10.5,
                pos_y: 20.5,
            })
            .await
            .expect("node create should succeed");

        let found = repo
            .find_by_id(&created.id)
            .await
            .expect("find_by_id should succeed")
            .expect("created node should exist");
        assert_eq!(found.label, "10.0.0.1");
        assert_eq!(found.description, "Initial node");
        assert_eq!(found.confidence, 0.7);

        let updated = repo
            .update(
                &created.id,
                UpdateNodeData {
                    label: Some("10.0.0.2".to_string()),
                    description: Some("Updated node".to_string()),
                    confidence: Some(0.9),
                    properties: Some(r#"{"type":"ip_address","updated":true}"#.to_string()),
                    pos_x: Some(42.0),
                    pos_y: Some(84.0),
                },
            )
            .await
            .expect("update should succeed");
        assert_eq!(updated.label, "10.0.0.2");
        assert_eq!(updated.description, "Updated node");
        assert_eq!(updated.confidence, 0.9);
        assert_eq!(updated.pos_x, 42.0);
        assert_eq!(updated.pos_y, 84.0);

        repo.delete(&created.id)
            .await
            .expect("delete should succeed");

        let deleted = repo
            .find_by_id(&created.id)
            .await
            .expect("find after delete should succeed");
        assert!(deleted.is_none());
    }
}
