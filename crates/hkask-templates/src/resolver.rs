//! Template Resolver — Maps template IDs to paths via registry
//!
//! Decouples manifests from filesystem paths, enabling registry abstraction.
//! Per architecture v0.21.0: Loose coupling via indirection.
//! Per Planck minimalism: No cache — registry lookups are O(1) SQLite queries.

use crate::ports::{RegistryIndex, Result};

/// Template resolver — direct registry lookup without caching
/// 
/// Following Planck's constant minimalism: remove state when function can be computed directly.
/// Registry lookups are O(1) SQLite queries — caching adds complexity without measurable benefit.
pub struct TemplateResolver<R> {
    registry: R,
}

impl<R: RegistryIndex> TemplateResolver<R> {
    /// Create new resolver
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    /// Resolve template ID to path via direct registry lookup
    pub fn resolve(&self, template_id: &str) -> Result<String> {
        // Direct registry lookup — O(1) SQLite query
        let entry = self.registry.get(template_id)?;
        Ok(entry.source_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{RegistryEntry, RegistryIndex, Result, TemplateError};
    use hkask_types::TemplateType;

    struct MockRegistry {
        entries: HashMap<String, RegistryEntry>,
    }

    impl MockRegistry {
        fn new() -> Self {
            Self {
                entries: HashMap::new(),
            }
        }

        fn add(&mut self, entry: RegistryEntry) {
            self.entries.insert(entry.id.clone(), entry);
        }
    }

    impl RegistryIndex for MockRegistry {
        fn get(&self, id: &str) -> Result<RegistryEntry> {
            self.entries
                .get(id)
                .cloned()
                .ok_or_else(|| TemplateError::NotFound(id.to_string()))
        }

        fn list(&self, _template_type: Option<TemplateType>) -> Vec<RegistryEntry> {
            self.entries.values().cloned().collect()
        }
    }

    #[test]
    fn test_resolver_resolve() {
        let mut registry = MockRegistry::new();
        registry.add(RegistryEntry {
            id: "test/template".to_string(),
            template_type: TemplateType::Prompt,
            lexicon_terms: vec![],
            description: "Test".to_string(),
            source_path: "/path/to/template.jinja2".to_string(),
        });

        let resolver = TemplateResolver::new(registry);

        // Direct lookup (no cache)
        let path = resolver.resolve("test/template").unwrap();
        assert_eq!(path, "/path/to/template.jinja2");
    }

    #[test]
    fn test_resolver_multiple_lookups() {
        let mut registry = MockRegistry::new();
        registry.add(RegistryEntry {
            id: "test/template1".to_string(),
            template_type: TemplateType::Prompt,
            lexicon_terms: vec![],
            description: "Test".to_string(),
            source_path: "/path/to/template1.jinja2".to_string(),
        });
        registry.add(RegistryEntry {
            id: "test/template2".to_string(),
            template_type: TemplateType::Prompt,
            lexicon_terms: vec![],
            description: "Test".to_string(),
            source_path: "/path/to/template2.jinja2".to_string(),
        });

        let resolver = TemplateResolver::new(registry);

        // Multiple lookups should all succeed
        let path1 = resolver.resolve("test/template1").unwrap();
        let path2 = resolver.resolve("test/template2").unwrap();

        assert_eq!(path1, "/path/to/template1.jinja2");
        assert_eq!(path2, "/path/to/template2.jinja2");
    }

    #[test]
    fn test_resolver_not_found() {
        let registry = MockRegistry::new();
        let resolver = TemplateResolver::new(registry);

        let result = resolver.resolve("nonexistent/template");
        assert!(result.is_err());
    }
}

    impl MockRegistry {
        fn new() -> Self {
            Self {
                entries: HashMap::new(),
            }
        }

        fn add(&mut self, entry: RegistryEntry) {
            self.entries.insert(entry.id.clone(), entry);
        }
    }

    impl RegistryIndex for MockRegistry {
        fn get(&self, id: &str) -> Result<RegistryEntry> {
            self.entries
                .get(id)
                .cloned()
                .ok_or_else(|| TemplateError::NotFound(id.to_string()))
        }

        fn list(&self, _template_type: Option<TemplateType>) -> Vec<RegistryEntry> {
            self.entries.values().cloned().collect()
        }

        fn bootstrap_manifest(&self) -> Option<crate::ports::ProcessManifest> {
            None
        }
    }

    #[test]
    fn test_resolver_cache_hit() {
        let mut registry = MockRegistry::new();
        registry.add(RegistryEntry {
            id: "test/template".to_string(),
            template_type: TemplateType::Prompt,
            lexicon_terms: vec!["test".to_string()],
            description: "Test template".to_string(),
            source_path: "/path/to/template.jinja2".to_string(),
        });

        let mut resolver = TemplateResolver::new(registry);

        // First lookup (cache miss)
        let path1 = resolver.resolve("test/template").unwrap();
        assert_eq!(path1, "/path/to/template.jinja2");

        // Second lookup (cache hit)
        let path2 = resolver.resolve("test/template").unwrap();
        assert_eq!(path2, "/path/to/template.jinja2");
    }

    #[test]
    fn test_resolver_cache_ttl() {
        let mut registry = MockRegistry::new();
        registry.add(RegistryEntry {
            id: "test/template".to_string(),
            template_type: TemplateType::Prompt,
            lexicon_terms: vec![],
            description: "Test".to_string(),
            source_path: "/path/to/template.jinja2".to_string(),
        });

        let mut resolver = TemplateResolver::new(registry).with_ttl(Duration::from_millis(100));

        // First lookup
        resolver.resolve("test/template").unwrap();

        // Wait for TTL to expire
        std::thread::sleep(Duration::from_millis(150));

        // Lookup after TTL (should be cache miss, but still succeed)
        let path = resolver.resolve("test/template").unwrap();
        assert_eq!(path, "/path/to/template.jinja2");
    }

    #[test]
    fn test_resolver_cache_stats() {
        let mut registry = MockRegistry::new();
        registry.add(RegistryEntry {
            id: "test/template1".to_string(),
            template_type: TemplateType::Prompt,
            lexicon_terms: vec![],
            description: "Test".to_string(),
            source_path: "/path/to/template1.jinja2".to_string(),
        });
        registry.add(RegistryEntry {
            id: "test/template2".to_string(),
            template_type: TemplateType::Prompt,
            lexicon_terms: vec![],
            description: "Test".to_string(),
            source_path: "/path/to/template2.jinja2".to_string(),
        });

        let mut resolver = TemplateResolver::new(registry);

        // Resolve both templates
        resolver.resolve("test/template1").unwrap();
        resolver.resolve("test/template2").unwrap();

        let stats = resolver.cache_stats();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.valid_entries, 2);
        assert_eq!(stats.expired_entries, 0);
    }

    #[test]
    fn test_resolver_clear_cache() {
        let mut registry = MockRegistry::new();
        registry.add(RegistryEntry {
            id: "test/template".to_string(),
            template_type: TemplateType::Prompt,
            lexicon_terms: vec![],
            description: "Test".to_string(),
            source_path: "/path/to/template.jinja2".to_string(),
        });

        let mut resolver = TemplateResolver::new(registry);

        // Resolve and cache
        resolver.resolve("test/template").unwrap();
        assert_eq!(resolver.cache_stats().valid_entries, 1);

        // Clear cache
        resolver.clear_cache();
        assert_eq!(resolver.cache_stats().total_entries, 0);
    }

    #[test]
    fn test_resolver_not_found() {
        let registry = MockRegistry::new();
        let mut resolver = TemplateResolver::new(registry);

        let result = resolver.resolve("nonexistent/template");
        assert!(result.is_err());
    }
}
