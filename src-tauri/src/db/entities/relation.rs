use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "relations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub investigation_id: String,
    pub relation_type: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub label: String,
    pub confidence: f32,
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
    pub properties: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::investigation::Entity",
        from = "Column::InvestigationId",
        to = "super::investigation::Column::Id"
    )]
    Investigation,
    #[sea_orm(
        belongs_to = "super::node::Entity",
        from = "Column::SourceNodeId",
        to = "super::node::Column::Id"
    )]
    SourceNode,
    #[sea_orm(
        belongs_to = "super::node::Entity",
        from = "Column::TargetNodeId",
        to = "super::node::Column::Id"
    )]
    TargetNode,
}

impl Related<super::investigation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Investigation.def()
    }
}

impl Related<super::node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SourceNode.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
