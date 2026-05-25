use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "nodes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub investigation_id: String,
    pub node_type: String,
    pub label: String,
    pub description: String,
    pub confidence: f32,
    pub properties: String,
    pub pos_x: f64,
    pub pos_y: f64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::investigation::Entity",
        from = "Column::InvestigationId",
        to = "super::investigation::Column::Id"
    )]
    Investigation,
    #[sea_orm(has_many = "super::relation::Entity")]
    OutgoingRelations,
    #[sea_orm(has_many = "super::relation::Entity")]
    IncomingRelations,
}

impl Related<super::investigation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Investigation.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
