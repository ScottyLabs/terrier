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

impl FederationIndex {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
        }
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

    async fn refresh(&self) -> anyhow::Result<()> {
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
fn parse_idp_entries(xml: &str) -> Vec<EntityEntry> {
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
fn entity_descriptor_fragments(xml: &str) -> Vec<&str> {
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

/// Fetches the InCommon aggregate on startup and refreshes it every 6 hours.
pub async fn federation_index_task(index: FederationIndex) {
    loop {
        if let Err(e) = index.refresh().await {
            tracing::error!(error = %e, "failed to refresh federation index");
        }
        tokio::time::sleep(REFRESH_INTERVAL).await;
    }
}
