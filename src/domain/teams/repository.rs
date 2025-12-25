#[cfg(feature = "server")]
use crate::core::database::repository::Repository;
#[cfg(feature = "server")]
use crate::entities::prelude::*;
#[cfg(feature = "server")]
use crate::entities::{teams, user_hackathon_roles};
#[cfg(feature = "server")]
use dioxus::prelude::ServerFnError;
#[cfg(feature = "server")]
use sea_orm::{ColumnTrait, DatabaseConnection, QueryFilter};

#[cfg(feature = "server")]
use super::types::*;

#[cfg(feature = "server")]
pub struct TeamRepository<'a> {
    repo: Repository<'a>,
}

#[cfg(feature = "server")]
impl<'a> TeamRepository<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self {
            repo: Repository::new(db),
        }
    }

    /// Find team by ID
    pub async fn find_by_id(&self, team_id: i32) -> Result<teams::Model, ServerFnError> {
        self.repo.find_by_id::<Teams>(team_id).await
    }

    /// Get user's role in a hackathon
    pub async fn find_user_role(
        &self,
        user_id: i32,
        hackathon_id: i32,
    ) -> Result<Option<user_hackathon_roles::Model>, ServerFnError> {
        self.repo
            .find_one::<UserHackathonRoles, _>(|query| {
                query
                    .filter(user_hackathon_roles::Column::UserId.eq(user_id))
                    .filter(user_hackathon_roles::Column::HackathonId.eq(hackathon_id))
            })
            .await
    }

    /// Get user's role in a hackathon or return error
    pub async fn find_user_role_or_error(
        &self,
        user_id: i32,
        hackathon_id: i32,
        error_msg: &str,
    ) -> Result<user_hackathon_roles::Model, ServerFnError> {
        self.repo
            .find_one_or_error::<UserHackathonRoles, _>(
                |query| {
                    query
                        .filter(user_hackathon_roles::Column::UserId.eq(user_id))
                        .filter(user_hackathon_roles::Column::HackathonId.eq(hackathon_id))
                },
                error_msg,
            )
            .await
    }

    /// Get user's team for a hackathon
    pub async fn find_user_team(
        &self,
        user_id: i32,
        hackathon_id: i32,
    ) -> Result<Option<i32>, ServerFnError> {
        let role = self.find_user_role(user_id, hackathon_id).await?;
        Ok(role.and_then(|r| r.team_id))
    }

    /// Get a specific user's role within a team
    pub async fn find_team_member_role(
        &self,
        user_id: i32,
        hackathon_id: i32,
        team_id: i32,
    ) -> Result<Option<user_hackathon_roles::Model>, ServerFnError> {
        self.repo
            .find_one::<UserHackathonRoles, _>(|query| {
                query
                    .filter(user_hackathon_roles::Column::UserId.eq(user_id))
                    .filter(user_hackathon_roles::Column::HackathonId.eq(hackathon_id))
                    .filter(user_hackathon_roles::Column::TeamId.eq(team_id))
            })
            .await
    }

    /// Get a specific user's role within a team or return error
    pub async fn find_team_member_role_or_error(
        &self,
        user_id: i32,
        hackathon_id: i32,
        team_id: i32,
        error_msg: &str,
    ) -> Result<user_hackathon_roles::Model, ServerFnError> {
        self.repo
            .find_one_or_error::<UserHackathonRoles, _>(
                |query| {
                    query
                        .filter(user_hackathon_roles::Column::UserId.eq(user_id))
                        .filter(user_hackathon_roles::Column::HackathonId.eq(hackathon_id))
                        .filter(user_hackathon_roles::Column::TeamId.eq(team_id))
                },
                error_msg,
            )
            .await
    }

    /// Get all team member roles (without user data)
    pub async fn get_team_member_roles(
        &self,
        team_id: i32,
        hackathon_id: i32,
    ) -> Result<Vec<user_hackathon_roles::Model>, ServerFnError> {
        self.repo
            .find_all::<UserHackathonRoles, _>(|query| {
                query
                    .filter(user_hackathon_roles::Column::TeamId.eq(team_id))
                    .filter(user_hackathon_roles::Column::HackathonId.eq(hackathon_id))
            })
            .await
    }

    /// Get all team members with their user data
    pub async fn get_team_members(
        &self,
        team_id: i32,
        hackathon_id: i32,
    ) -> Result<Vec<TeamMemberData>, ServerFnError> {
        let member_roles = self.get_team_member_roles(team_id, hackathon_id).await?;

        // Fetch user data for each member
        let mut members = Vec::new();
        for role in member_roles {
            let user = self.repo.find_by_id::<Users>(role.user_id).await?;
            members.push(TeamMemberData {
                user_id: user.id,
                name: user.name,
                email: user.email,
                picture: user.picture,
            });
        }

        Ok(members)
    }

    /// Count team members
    pub async fn count_team_members(
        &self,
        team_id: i32,
        hackathon_id: i32,
    ) -> Result<usize, ServerFnError> {
        let members = self.get_team_member_roles(team_id, hackathon_id).await?;
        Ok(members.len())
    }

    /// Get team with full member data
    pub async fn get_team_with_members(
        &self,
        team_id: i32,
        hackathon_id: i32,
        current_user_id: i32,
        max_team_size: i32,
    ) -> Result<TeamData, ServerFnError> {
        let team = self.find_by_id(team_id).await?;
        let mut members = self.get_team_members(team_id, hackathon_id).await?;

        // Sort members so owner is first
        members.sort_by_key(|m| if m.user_id == team.owner_id { 0 } else { 1 });

        Ok(TeamData {
            id: team.id,
            name: team.name,
            description: team.description,
            member_count: members.len(),
            max_size: max_team_size,
            is_owner: team.owner_id == current_user_id,
            is_member: true,
            members,
        })
    }
}
