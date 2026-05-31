//! STIX 2.x bundle importer — converts STIX objects into domain nodes and relations.

use crate::error::AppError;
use crate::models::domain::{NodeData, RelationData};
use crate::models::stix::{parse_stix_bundle, StixBundle};

pub fn import_stix_json(json: &str) -> Result<(Vec<NodeData>, Vec<RelationData>), AppError> {
    let bundle: StixBundle = serde_json::from_str(json)?;
    parse_stix_bundle(bundle)
}
