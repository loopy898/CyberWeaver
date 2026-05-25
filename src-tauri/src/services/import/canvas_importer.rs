//! tldraw canvas importer — extracts graph data from canvas JSON.

use crate::error::AppError;
use crate::models::canvas_format::{parse_json_canvas, JsonCanvas};
use crate::models::domain::{NodeData, RelationData};

pub fn import_canvas_json(json: &str) -> Result<(Vec<NodeData>, Vec<RelationData>), AppError> {
    let canvas: JsonCanvas = serde_json::from_str(json)?;
    parse_json_canvas(canvas)
}
