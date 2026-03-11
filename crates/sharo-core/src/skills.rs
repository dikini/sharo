use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillSourceScope {
    Project,
    User,
    Configured,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCatalogEntry {
    pub skill_id: String,
    pub name: String,
    pub description: String,
    pub source_scope: SkillSourceScope,
    pub trust_label: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDocument {
    pub skill_id: String,
    pub name: String,
    pub description: String,
    pub source_scope: SkillSourceScope,
    pub trust_label: String,
    pub markdown: String,
    pub has_scripts: bool,
    pub has_references: bool,
    pub has_assets: bool,
}

pub fn derive_skill_id(root: &Path, skill_dir: &Path) -> Option<String> {
    let relative = skill_dir.strip_prefix(root).ok()?;
    let segments = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    (!segments.is_empty()).then(|| segments.join("/"))
}
