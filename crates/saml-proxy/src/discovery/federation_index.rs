use samael::metadata::EntityDescriptor;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

const AGGREGATE_URL: &str = "https://mdq.incommon.org/entities";
const REFRESH_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Clone, serde::Serialize)]
pub struct EntityEntry {
    pub entity_id: String,
    pub display_name: String,
}

/// In-memory index of InCommon federation IdP entities, refreshed periodically
/// from the MDQ aggregate endpoint. Parses individual EntityDescriptor fragments
/// rather than deserializing the full aggregate into a DOM.
#[derive(Clone)]
pub struct FederationIndex {
    entries: Arc<RwLock<Vec<EntityEntry>>>,
}

impl Default for FederationIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl FederationIndex {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn entries(&self) -> &Arc<RwLock<Vec<EntityEntry>>> {
        &self.entries
    }

    pub async fn search(&self, query: &str, limit: usize) -> Vec<EntityEntry> {
        let query_lower = query.to_lowercase();
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.display_name.to_lowercase().contains(&query_lower))
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn refresh(&self) -> anyhow::Result<()> {
        let xml = reqwest::get(AGGREGATE_URL).await?.text().await?;
        let entries = parse_idp_entries(&xml);

        let count = entries.len();
        *self.entries.write().await = entries;
        tracing::info!(count, "refreshed federation index");

        Ok(())
    }
}

/// Splits the aggregate XML into individual EntityDescriptor fragments, skips
/// any that lack an IDPSSODescriptor, then parses the remaining ones with
/// samael to extract entity ID and display name.
pub(crate) fn parse_idp_entries(xml: &str) -> Vec<EntityEntry> {
    let mut entries = Vec::new();

    for fragment in entity_descriptor_fragments(xml) {
        if !fragment.contains("IDPSSODescriptor") {
            continue;
        }

        let entity: EntityDescriptor = match fragment.parse() {
            Ok(e) => e,
            Err(e) => {
                tracing::debug!(error = %e, "skipping unparseable EntityDescriptor");
                continue;
            }
        };

        let entity_id = match entity.entity_id {
            Some(id) => id,
            None => continue,
        };

        let display_name = entity
            .organization
            .as_ref()
            .and_then(|org| org.organization_display_names.as_ref())
            .and_then(|names| {
                names
                    .iter()
                    .find(|n| n.lang.as_deref() == Some("en"))
                    .or(names.first())
            })
            .map(|n| n.value.clone())
            .unwrap_or_else(|| entity_id.clone());

        entries.push(EntityEntry {
            entity_id,
            display_name,
        });
    }

    entries.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    entries
}

/// Yields the substring of each top-level <EntityDescriptor ...>...</EntityDescriptor>
/// element in the aggregate XML.
pub(crate) fn entity_descriptor_fragments(xml: &str) -> Vec<&str> {
    let mut fragments = Vec::new();
    let mut search_from = 0;

    while let Some(start) = xml[search_from..].find("<EntityDescriptor") {
        let abs_start = search_from + start;
        let close_tag = "</EntityDescriptor>";
        if let Some(end) = xml[abs_start..].find(close_tag) {
            let abs_end = abs_start + end + close_tag.len();
            fragments.push(&xml[abs_start..abs_end]);
            search_from = abs_end;
        } else {
            break;
        }
    }

    fragments
}

#[cfg(test)]
mod tests {
    use super::*;

    const AGGREGATE_XML: &str = r#"<EntitiesDescriptor xmlns="urn:oasis:names:tc:SAML:2.0:metadata">
<EntityDescriptor entityID="https://idp.example.edu">
    <IDPSSODescriptor protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
        <SingleSignOnService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect" Location="https://idp.example.edu/sso"/>
    </IDPSSODescriptor>
    <Organization>
        <OrganizationDisplayName xml:lang="en">Example University</OrganizationDisplayName>
    </Organization>
</EntityDescriptor>
<EntityDescriptor entityID="https://sp.example.com">
    <SPSSODescriptor protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
        <AssertionConsumerService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" Location="https://sp.example.com/acs" index="0"/>
    </SPSSODescriptor>
</EntityDescriptor>
<EntityDescriptor entityID="https://idp2.example.org">
    <IDPSSODescriptor protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
        <SingleSignOnService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect" Location="https://idp2.example.org/sso"/>
    </IDPSSODescriptor>
</EntityDescriptor>
</EntitiesDescriptor>"#;

    #[test]
    fn fragments_splits_entity_descriptors() {
        let fragments = entity_descriptor_fragments(AGGREGATE_XML);
        assert_eq!(fragments.len(), 3);
        assert!(fragments[0].contains("idp.example.edu"));
        assert!(fragments[1].contains("sp.example.com"));
        assert!(fragments[2].contains("idp2.example.org"));
    }

    #[test]
    fn fragments_empty_input() {
        let fragments = entity_descriptor_fragments("");
        assert!(fragments.is_empty());
    }

    #[test]
    fn fragments_no_descriptors() {
        let fragments = entity_descriptor_fragments("<EntitiesDescriptor></EntitiesDescriptor>");
        assert!(fragments.is_empty());
    }

    #[test]
    fn parse_filters_to_idp_only() {
        let entries = parse_idp_entries(AGGREGATE_XML);
        // SP-only entity (sp.example.com) should be filtered out
        assert_eq!(entries.len(), 2);
        assert!(
            entries
                .iter()
                .all(|e| e.entity_id != "https://sp.example.com")
        );
    }

    #[test]
    fn parse_extracts_display_name() {
        let entries = parse_idp_entries(AGGREGATE_XML);
        let example = entries
            .iter()
            .find(|e| e.entity_id == "https://idp.example.edu")
            .unwrap();
        assert_eq!(example.display_name, "Example University");
    }

    #[test]
    fn parse_falls_back_to_entity_id() {
        let entries = parse_idp_entries(AGGREGATE_XML);
        // idp2.example.org has no Organization element, should fall back to entity ID
        let idp2 = entries
            .iter()
            .find(|e| e.entity_id == "https://idp2.example.org")
            .unwrap();
        assert_eq!(idp2.display_name, "https://idp2.example.org");
    }

    #[test]
    fn parse_results_sorted_alphabetically() {
        let entries = parse_idp_entries(AGGREGATE_XML);
        let names: Vec<&str> = entries.iter().map(|e| e.display_name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[tokio::test]
    async fn search_filters_by_query() {
        let index = FederationIndex::new();
        {
            let mut entries = index.entries.write().await;
            *entries = vec![
                EntityEntry {
                    entity_id: "https://a.edu".into(),
                    display_name: "Alpha University".into(),
                },
                EntityEntry {
                    entity_id: "https://b.edu".into(),
                    display_name: "Beta College".into(),
                },
                EntityEntry {
                    entity_id: "https://c.edu".into(),
                    display_name: "Alpha Tech".into(),
                },
            ];
        }

        let results = index.search("alpha", 20).await;
        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .all(|e| e.display_name.to_lowercase().contains("alpha"))
        );
    }

    #[tokio::test]
    async fn search_respects_limit() {
        let index = FederationIndex::new();
        {
            let mut entries = index.entries.write().await;
            *entries = (0..30)
                .map(|i| EntityEntry {
                    entity_id: format!("https://{i}.edu"),
                    display_name: format!("University {i}"),
                })
                .collect();
        }

        let results = index.search("University", 5).await;
        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn search_case_insensitive() {
        let index = FederationIndex::new();
        {
            let mut entries = index.entries.write().await;
            *entries = vec![EntityEntry {
                entity_id: "https://cmu.edu".into(),
                display_name: "Carnegie Mellon University".into(),
            }];
        }

        let results = index.search("CARNEGIE", 20).await;
        assert_eq!(results.len(), 1);
    }
}

/// Fetches the InCommon aggregate on startup and refreshes it every 6 hours.
pub async fn federation_index_task(index: FederationIndex) {
    loop {
        if let Err(e) = index.refresh().await {
            tracing::error!(error = %e, "failed to refresh federation index");
        }
        tokio::time::sleep(REFRESH_INTERVAL).await;
    }
}
