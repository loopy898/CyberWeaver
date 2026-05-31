use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::db::entities::relation;
use crate::error::AppError;

pub struct RelationRepo<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> RelationRepo<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// 获取调查案件中的所有关系
    pub async fn find_by_investigation(
        &self,
        investigation_id: &str,
    ) -> Result<Vec<relation::Model>, AppError> {
        let relations = relation::Entity::find()
            .filter(relation::Column::InvestigationId.eq(investigation_id))
            .all(self.db)
            .await?;
        Ok(relations)
    }

    /// 获取某个节点的所有出边
    pub async fn find_outgoing(&self, node_id: &str) -> Result<Vec<relation::Model>, AppError> {
        let relations = relation::Entity::find()
            .filter(relation::Column::SourceNodeId.eq(node_id))
            .all(self.db)
            .await?;
        Ok(relations)
    }

    /// 获取某个节点的所有入边
    pub async fn find_incoming(&self, node_id: &str) -> Result<Vec<relation::Model>, AppError> {
        let relations = relation::Entity::find()
            .filter(relation::Column::TargetNodeId.eq(node_id))
            .all(self.db)
            .await?;
        Ok(relations)
    }

    /// 获取两个节点之间的所有关系
    pub async fn find_between(
        &self,
        node_a: &str,
        node_b: &str,
    ) -> Result<Vec<relation::Model>, AppError> {
        let relations = relation::Entity::find()
            .filter(
                relation::Column::SourceNodeId
                    .eq(node_a)
                    .and(relation::Column::TargetNodeId.eq(node_b)),
            )
            .all(self.db)
            .await?;
        Ok(relations)
    }

    /// 创建新关系
    pub async fn create(&self, data: CreateRelationData) -> Result<relation::Model, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = crate::db::repositories::node_repo::chrono_now();

        let active = relation::ActiveModel {
            id: Set(id),
            investigation_id: Set(data.investigation_id),
            relation_type: Set(data.relation_type),
            source_node_id: Set(data.source_node_id),
            target_node_id: Set(data.target_node_id),
            label: Set(data.label),
            confidence: Set(data.confidence),
            first_seen: Set(data.first_seen),
            last_seen: Set(data.last_seen),
            properties: Set(data.properties),
            created_at: Set(now),
        };

        let model = active.insert(self.db).await?;
        Ok(model)
    }

    /// 删除关系
    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        relation::Entity::delete_by_id(id).exec(self.db).await?;
        Ok(())
    }

    /// 删除节点的所有关联关系
    pub async fn delete_by_node(&self, node_id: &str) -> Result<(), AppError> {
        relation::Entity::delete_many()
            .filter(
                relation::Column::SourceNodeId
                    .eq(node_id)
                    .or(relation::Column::TargetNodeId.eq(node_id)),
            )
            .exec(self.db)
            .await?;
        Ok(())
    }
}

pub struct CreateRelationData {
    pub investigation_id: String,
    pub relation_type: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub label: String,
    pub confidence: f32,
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
    pub properties: String,
}
