//! Attack Flow Builder (AFB) importer — converts AFB JSON to domain model.

use crate::error::AppError;
use crate::models::attack_flow::{parse_attack_flow, AfbBundle};
use crate::models::domain::{NodeData, RelationData};

pub fn import_afb_json(json: &str) -> Result<(Vec<NodeData>, Vec<RelationData>), AppError> {
    let bundle: AfbBundle = serde_json::from_str(json)?;
    parse_attack_flow(bundle)
}
