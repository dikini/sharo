use crate::reasoning_context::TurnScope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentProvenance {
    pub source: String,
    pub applied_filters: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedComponent {
    pub content: String,
    pub provenance: ComponentProvenance,
}

pub trait ComponentResolver: Send + Sync {
    fn resolve(&self, scope: &TurnScope) -> Result<ResolvedComponent, String>;
}

#[derive(Debug, Clone)]
pub struct StaticTextResolver {
    content: String,
    source: String,
}

impl StaticTextResolver {
    pub fn new(content: &str, source: &str) -> Self {
        Self {
            content: content.to_string(),
            source: source.to_string(),
        }
    }
}

impl ComponentResolver for StaticTextResolver {
    fn resolve(&self, _scope: &TurnScope) -> Result<ResolvedComponent, String> {
        Ok(ResolvedComponent {
            content: self.content.clone(),
            provenance: ComponentProvenance {
                source: self.source.clone(),
                applied_filters: vec![],
            },
        })
    }
}

pub struct ResolverBundle {
    pub system: Box<dyn ComponentResolver>,
    pub persona: Box<dyn ComponentResolver>,
    pub memory: Box<dyn ComponentResolver>,
    pub runtime: Box<dyn ComponentResolver>,
}

impl Default for ResolverBundle {
    fn default() -> Self {
        Self {
            system: Box::new(StaticTextResolver::new("", "default-system")),
            persona: Box::new(StaticTextResolver::new("", "default-persona")),
            memory: Box::new(StaticTextResolver::new("", "default-memory")),
            runtime: Box::new(StaticTextResolver::new("", "default-runtime")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedContext {
    pub system: ResolvedComponent,
    pub persona: ResolvedComponent,
    pub memory: ResolvedComponent,
    pub runtime: ResolvedComponent,
}

fn apply_component_local_filters(mut component: ResolvedComponent) -> ResolvedComponent {
    let trimmed = component.content.trim().to_string();
    if trimmed != component.content {
        component.provenance.applied_filters.push("trim_whitespace".to_string());
        component.content = trimmed;
    }
    component
}

pub fn resolve_context(bundle: &ResolverBundle, scope: &TurnScope) -> Result<ResolvedContext, String> {
    Ok(ResolvedContext {
        system: apply_component_local_filters(bundle.system.resolve(scope)?),
        persona: apply_component_local_filters(bundle.persona.resolve(scope)?),
        memory: apply_component_local_filters(bundle.memory.resolve(scope)?),
        runtime: apply_component_local_filters(bundle.runtime.resolve(scope)?),
    })
}
