use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use sharo_core::skills::{SkillCatalogEntry, SkillDocument, SkillSourceScope, derive_skill_id};

use crate::config::SkillsConfig;

const MAX_SKILL_CATALOG_ENTRIES: usize = 100;
const MAX_SKILL_MARKDOWN_CHARS: usize = 65_536;

#[derive(Debug, Clone, Default)]
pub struct SkillCatalog {
    skills: BTreeMap<String, SkillDocument>,
}

impl SkillCatalog {
    pub fn list(&self, active_skill_ids: &[String]) -> Vec<SkillCatalogEntry> {
        let active = active_skill_ids.iter().cloned().collect::<BTreeSet<_>>();
        self.skills
            .values()
            .take(MAX_SKILL_CATALOG_ENTRIES)
            .map(|skill| SkillCatalogEntry {
                skill_id: skill.skill_id.clone(),
                name: skill.name.clone(),
                description: skill.description.clone(),
                source_scope: skill.source_scope.clone(),
                trust_label: skill.trust_label.clone(),
                is_active: active.contains(&skill.skill_id),
            })
            .collect()
    }

    pub fn get(&self, skill_id: &str) -> Result<Option<SkillDocument>, String> {
        match self.skills.get(skill_id) {
            Some(skill) if skill.markdown.chars().count() > MAX_SKILL_MARKDOWN_CHARS => {
                Err(format!(
                    "skill_payload_too_large skill_id={} max_chars={}",
                    skill_id, MAX_SKILL_MARKDOWN_CHARS
                ))
            }
            Some(skill) => Ok(Some(skill.clone())),
            None => Ok(None),
        }
    }

    pub fn validate_skill_ids(&self, requested: &[String]) -> Result<Vec<String>, String> {
        let deduped = requested.iter().cloned().collect::<BTreeSet<_>>();
        if let Some(missing) = deduped
            .iter()
            .find(|skill_id| !self.skills.contains_key(skill_id.as_str()))
        {
            return Err(format!("skill_not_found skill_id={missing}"));
        }
        Ok(deduped.iter().cloned().collect())
    }
}

pub fn load_skill_catalog(config: &SkillsConfig) -> Result<SkillCatalog, String> {
    let mut catalog = SkillCatalog::default();
    for root in resolve_skill_roots(config)? {
        discover_skills_in_root(
            &root.path,
            &root.source,
            &root.trust_label,
            root.max_depth,
            &mut catalog,
        )?;
    }
    Ok(catalog)
}

struct ResolvedSkillRoot {
    path: PathBuf,
    source: SkillSourceScope,
    trust_label: String,
    max_depth: usize,
}

fn resolve_skill_roots(config: &SkillsConfig) -> Result<Vec<ResolvedSkillRoot>, String> {
    let mut roots = Vec::new();
    let max_depth = config.max_depth.unwrap_or(5);

    if config.enable_project_skills.unwrap_or(true) && config.trust_project_skills.unwrap_or(true) {
        let project_root = match &config.project_root {
            Some(path) => PathBuf::from(path),
            None => std::env::current_dir()
                .map_err(|e| format!("skills_project_root_resolve_failed error={e}"))?
                .join(".agents")
                .join("skills"),
        };
        roots.push(ResolvedSkillRoot {
            path: project_root,
            source: SkillSourceScope::Project,
            trust_label: "project".to_string(),
            max_depth,
        });
    }

    if config.enable_user_skills.unwrap_or(true) {
        let user_root = match &config.user_root {
            Some(path) => Some(PathBuf::from(path)),
            None => std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".agents").join("skills")),
        };
        if let Some(user_root) = user_root {
            roots.push(ResolvedSkillRoot {
                path: user_root,
                source: SkillSourceScope::User,
                trust_label: "user".to_string(),
                max_depth,
            });
        }
    }

    for root in &config.roots {
        roots.push(ResolvedSkillRoot {
            path: PathBuf::from(root),
            source: SkillSourceScope::Configured,
            trust_label: "configured".to_string(),
            max_depth,
        });
    }

    Ok(roots)
}

fn discover_skills_in_root(
    root: &Path,
    source: &SkillSourceScope,
    trust_label: &str,
    max_depth: usize,
    catalog: &mut SkillCatalog,
) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    walk_skill_dir(root, root, source, trust_label, max_depth, 0, catalog)
}

fn walk_skill_dir(
    root: &Path,
    current: &Path,
    source: &SkillSourceScope,
    trust_label: &str,
    max_depth: usize,
    depth: usize,
    catalog: &mut SkillCatalog,
) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(current) {
        Ok(metadata) => metadata,
        Err(error) => {
            return Err(format!(
                "skills_metadata_failed path={} error={error}",
                current.display()
            ));
        }
    };
    if metadata.file_type().is_symlink() {
        return Ok(());
    }

    let skill_path = current.join("SKILL.md");
    if is_safe_skill_file(&skill_path)? {
        if let Some(document) = load_skill_document(root, current, source, trust_label)? {
            catalog
                .skills
                .entry(document.skill_id.clone())
                .or_insert(document);
        }
        return Ok(());
    }

    if depth >= max_depth {
        return Ok(());
    }

    let mut entries = fs::read_dir(current)
        .map_err(|error| {
            format!(
                "skills_read_dir_failed path={} error={error}",
                current.display()
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            format!(
                "skills_read_dir_failed path={} error={error}",
                current.display()
            )
        })?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let entry_type = entry.file_type().map_err(|error| {
            format!(
                "skills_entry_type_failed path={} error={error}",
                path.display()
            )
        })?;
        if !entry_type.is_dir() || entry_type.is_symlink() {
            continue;
        }
        let hidden = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with('.'));
        if hidden {
            continue;
        }
        walk_skill_dir(
            root,
            &path,
            source,
            trust_label,
            max_depth,
            depth + 1,
            catalog,
        )?;
    }

    Ok(())
}

fn is_safe_skill_file(skill_path: &Path) -> Result<bool, String> {
    let metadata = match fs::symlink_metadata(skill_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "skill_metadata_failed path={} error={error}",
                skill_path.display()
            ));
        }
    };
    Ok(metadata.file_type().is_file())
}

fn load_skill_document(
    root: &Path,
    skill_dir: &Path,
    source: &SkillSourceScope,
    trust_label: &str,
) -> Result<Option<SkillDocument>, String> {
    let Some(skill_id) = derive_skill_id(root, skill_dir) else {
        return Ok(None);
    };
    let markdown = fs::read_to_string(skill_dir.join("SKILL.md")).map_err(|error| {
        format!(
            "skill_read_failed path={} error={error}",
            skill_dir.join("SKILL.md").display()
        )
    })?;
    let metadata = parse_skill_metadata(&skill_id, &markdown);
    Ok(Some(SkillDocument {
        skill_id,
        name: metadata.name,
        description: metadata.description,
        source_scope: source.clone(),
        trust_label: trust_label.to_string(),
        markdown,
        has_scripts: skill_dir.join("scripts").is_dir(),
        has_references: skill_dir.join("references").is_dir(),
        has_assets: skill_dir.join("assets").is_dir(),
    }))
}

struct ParsedSkillMetadata {
    name: String,
    description: String,
}

fn parse_skill_metadata(skill_id: &str, markdown: &str) -> ParsedSkillMetadata {
    let (frontmatter, body) = split_frontmatter(markdown);
    let frontmatter_name = frontmatter
        .as_deref()
        .and_then(|text| find_frontmatter_value(text, "name"));
    let frontmatter_description = frontmatter
        .as_deref()
        .and_then(|text| find_frontmatter_value(text, "description"));
    let heading = first_heading(body);
    let paragraph = first_paragraph(body);
    let fallback_name = skill_id
        .rsplit('/')
        .next()
        .map(title_case_skill_name)
        .unwrap_or_else(|| "Unnamed Skill".to_string());

    ParsedSkillMetadata {
        name: frontmatter_name.or(heading).unwrap_or(fallback_name),
        description: frontmatter_description
            .or(paragraph)
            .unwrap_or_else(|| format!("Skill {skill_id}")),
    }
}

fn split_frontmatter(markdown: &str) -> (Option<String>, &str) {
    let start_len = if markdown.starts_with("---\n") {
        4
    } else if markdown.starts_with("---\r\n") {
        5
    } else {
        return (None, markdown);
    };
    let Some(stripped) = markdown.get(start_len..) else {
        return (None, markdown);
    };
    let mut offset = start_len;
    for line in stripped.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        offset += line.len();
        if trimmed == "---" {
            return (
                Some(markdown[start_len..offset - line.len()].to_string()),
                &markdown[offset..],
            );
        }
    }
    (None, markdown)
}

fn find_frontmatter_value(frontmatter: &str, key: &str) -> Option<String> {
    frontmatter.lines().find_map(|line| {
        let (raw_key, raw_value) = line.split_once(':')?;
        (raw_key.trim() == key).then(|| raw_value.trim().trim_matches('"').to_string())
    })
}

fn first_heading(markdown: &str) -> Option<String> {
    markdown.lines().find_map(|line| {
        line.trim()
            .strip_prefix("# ")
            .map(|value| value.trim().to_string())
    })
}

fn first_paragraph(markdown: &str) -> Option<String> {
    let mut paragraph = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        paragraph.push(trimmed);
    }
    (!paragraph.is_empty()).then(|| paragraph.join(" "))
}

fn title_case_skill_name(leaf: &str) -> String {
    leaf.split(['-', '_'])
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use sharo_core::skills::SkillSourceScope;

    use crate::config::SkillsConfig;
    use crate::skills::{load_skill_catalog, parse_skill_metadata};

    fn unique_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    fn write_skill(root: &PathBuf, relative_dir: &str, markdown: &str) {
        let skill_dir = root.join(relative_dir);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), markdown).expect("write skill");
    }

    fn config_for_roots(project_root: &PathBuf, user_root: &PathBuf) -> SkillsConfig {
        SkillsConfig {
            enable_project_skills: Some(true),
            enable_user_skills: Some(true),
            max_depth: Some(5),
            roots: vec![],
            project_root: Some(project_root.display().to_string()),
            user_root: Some(user_root.display().to_string()),
            trust_project_skills: Some(true),
        }
    }

    #[test]
    fn recursive_skill_discovery_finds_bundled_skill_layouts() {
        let project_root = unique_dir("sharo-skills-project");
        let user_root = unique_dir("sharo-skills-user");
        write_skill(
            &project_root,
            "writing/docs/strict-plan",
            "---\nname: Strict Plan\ndescription: Enforce plans\n---\n# Strict Plan\n",
        );

        let catalog = load_skill_catalog(&config_for_roots(&project_root, &user_root))
            .expect("load skill catalog");
        let skill = catalog
            .get("writing/docs/strict-plan")
            .expect("get skill")
            .expect("bundled skill");

        assert_eq!(skill.name, "Strict Plan");
        assert_eq!(skill.description, "Enforce plans");
        assert_eq!(skill.source_scope, SkillSourceScope::Project);

        let _ = fs::remove_dir_all(project_root);
        let _ = fs::remove_dir_all(user_root);
    }

    #[test]
    fn project_skill_precedence_overrides_user_scope() {
        let project_root = unique_dir("sharo-skills-project");
        let user_root = unique_dir("sharo-skills-user");
        write_skill(
            &project_root,
            "brainstorming",
            "---\nname: Project Brainstorming\ndescription: Project copy wins\n---\n# Project Brainstorming\n",
        );
        write_skill(
            &user_root,
            "brainstorming",
            "---\nname: User Brainstorming\ndescription: User copy loses\n---\n# User Brainstorming\n",
        );

        let catalog = load_skill_catalog(&config_for_roots(&project_root, &user_root))
            .expect("load skill catalog");
        let skill = catalog
            .get("brainstorming")
            .expect("get skill")
            .expect("skill");

        assert_eq!(skill.name, "Project Brainstorming");
        assert_eq!(skill.description, "Project copy wins");
        assert_eq!(skill.source_scope, SkillSourceScope::Project);

        let _ = fs::remove_dir_all(project_root);
        let _ = fs::remove_dir_all(user_root);
    }

    #[test]
    fn malformed_frontmatter_falls_back_to_heading_and_does_not_abort_discovery() {
        let metadata = parse_skill_metadata(
            "writing/broken",
            "---\nname Strict Plan\ndescription broken\n---\n# Heading Fallback\n\nFirst paragraph summary.\n",
        );
        assert_eq!(metadata.name, "Heading Fallback");
        assert_eq!(metadata.description, "First paragraph summary.");
    }

    #[test]
    fn crlf_frontmatter_is_parsed_leniently() {
        let metadata = parse_skill_metadata(
            "writing/crlf",
            "---\r\nname: CRLF Skill\r\ndescription: Handles windows newlines\r\n---\r\n# Heading\r\n",
        );
        assert_eq!(metadata.name, "CRLF Skill");
        assert_eq!(metadata.description, "Handles windows newlines");
    }

    #[test]
    fn untrusted_project_skills_are_not_disclosed() {
        let project_root = unique_dir("sharo-skills-project");
        let user_root = unique_dir("sharo-skills-user");
        write_skill(
            &project_root,
            "brainstorming",
            "---\nname: Project Brainstorming\ndescription: should be hidden\n---\n# Project Brainstorming\n",
        );
        let config = SkillsConfig {
            trust_project_skills: Some(false),
            ..config_for_roots(&project_root, &user_root)
        };

        let catalog = load_skill_catalog(&config).expect("load skill catalog");
        assert!(catalog.get("brainstorming").expect("get skill").is_none());

        let _ = fs::remove_dir_all(project_root);
        let _ = fs::remove_dir_all(user_root);
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_skill_markdown_is_ignored() {
        let project_root = unique_dir("sharo-skills-project");
        let user_root = unique_dir("sharo-skills-user");
        let outside = unique_dir("sharo-skills-outside");
        fs::create_dir_all(project_root.join("escaped")).expect("create escaped dir");
        fs::create_dir_all(&outside).expect("create outside dir");
        fs::write(outside.join("outside.md"), "# Escaped\n\nshould not load\n")
            .expect("write outside file");
        symlink(
            outside.join("outside.md"),
            project_root.join("escaped").join("SKILL.md"),
        )
        .expect("symlink skill markdown");

        let catalog = load_skill_catalog(&config_for_roots(&project_root, &user_root))
            .expect("load skill catalog");
        assert!(catalog.get("escaped").expect("get skill").is_none());

        let _ = fs::remove_dir_all(project_root);
        let _ = fs::remove_dir_all(user_root);
        let _ = fs::remove_dir_all(outside);
    }
}
