#[cfg(feature = "server")]
use crate::core::database::repository::Repository;
#[cfg(feature = "server")]
use crate::entities::hackathons;
#[cfg(feature = "server")]
use crate::entities::prelude::*;
#[cfg(feature = "server")]
use dioxus::prelude::ServerFnError;
#[cfg(feature = "server")]
use sea_orm::{ColumnTrait, DatabaseConnection, QueryFilter};

#[cfg(feature = "server")]
use super::types::HackathonInfo;

#[cfg(feature = "server")]
pub struct HackathonRepository<'a> {
    repo: Repository<'a>,
}

#[cfg(feature = "server")]
impl<'a> HackathonRepository<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self {
            repo: Repository::new(db),
        }
    }

    /// Find hackathon by slug
    pub async fn find_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<hackathons::Model>, ServerFnError> {
        self.repo
            .find_one::<Hackathons, _>(|query| query.filter(hackathons::Column::Slug.eq(slug)))
            .await
    }

    /// Find hackathon by slug or return error
    pub async fn find_by_slug_or_error(
        &self,
        slug: &str,
    ) -> Result<hackathons::Model, ServerFnError> {
        self.repo
            .find_one_or_error::<Hackathons, _>(
                |query| query.filter(hackathons::Column::Slug.eq(slug)),
                "Hackathon not found",
            )
            .await
    }

    /// Get all hackathons as domain types
    pub async fn get_all(&self) -> Result<Vec<HackathonInfo>, ServerFnError> {
        let hackathons = self.repo.find_all::<Hackathons, _>(|query| query).await?;
        Ok(hackathons.into_iter().map(|h| h.into()).collect())
    }
}

// Conversion from entity to domain type
#[cfg(feature = "server")]
impl From<hackathons::Model> for HackathonInfo {
    fn from(h: hackathons::Model) -> Self {
        HackathonInfo {
            id: h.id,
            name: h.name,
            slug: h.slug,
            description: h.description,
            start_date: h.start_date,
            end_date: h.end_date,
            is_active: h.is_active,
            max_team_size: h.max_team_size,
            banner_url: h.banner_url,
            background_url: h.background_url,
            updated_at: h.updated_at,
            form_config: h.form_config,
        }
    }
}
