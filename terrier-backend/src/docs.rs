use utoipa::{
    Modify, OpenApi,
    openapi::{
        SecurityRequirement,
        security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    },
};

use crate::{applications, auth, hackathons, teams};

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::handlers::status,
        auth::handlers::login,
        auth::handlers::logout,
        hackathons::handlers::list_public_hackathons,
        hackathons::handlers::get_user_role,
        hackathons::handlers::create_hackathon,
        applications::handlers::get_application,
        applications::handlers::save_application,
        applications::handlers::submit_application,
        applications::handlers::get_upload_url,
        teams::handlers::list_teams,
        teams::handlers::get_my_team,
        teams::handlers::get_team,
        teams::handlers::create_team,
        teams::handlers::update_team,
        teams::handlers::leave_team,
        teams::handlers::request_to_join,
        teams::handlers::get_join_requests,
        teams::handlers::respond_to_request,
        teams::handlers::invite_member,
        teams::handlers::respond_to_invite,
        teams::handlers::search_participants,
    ),
    components(schemas(
        auth::handlers::LoginQuery,
        auth::handlers::UserInfo,
        hackathons::handlers::HackathonInfo,
        hackathons::handlers::UserRoleResponse,
        hackathons::handlers::CreateHackathonRequest,
        applications::handlers::ApplicationResponse,
        applications::handlers::SaveApplicationRequest,
        applications::handlers::SaveApplicationResponse,
        applications::handlers::SubmitApplicationResponse,
        applications::handlers::UploadUrlRequest,
        applications::handlers::UploadUrlResponse,
        teams::handlers::TeamMemberInfo,
        teams::handlers::TeamResponse,
        teams::handlers::TeamListItem,
        teams::handlers::JoinRequestResponse,
        teams::handlers::TeamInviteResponse,
        teams::handlers::MyTeamResponse,
        teams::handlers::CreateTeamRequest,
        teams::handlers::UpdateTeamRequest,
        teams::handlers::JoinRequestRequest,
        teams::handlers::InviteMemberRequest,
        teams::handlers::RespondToRequestRequest,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "Hackathons", description = "Hackathon endpoints"),
        (name = "Authentication", description = "Authentication endpoints"),
        (name = "Applications", description = "Application form endpoints"),
        (name = "Teams", description = "Team management endpoints")
    ),
    info(
        title = "Terrier API",
        version = "1.0.0",
        description = "Terrier API",
        license(
            name = "MIT OR Apache-2.0",
            identifier = "MIT OR Apache-2.0"
        )
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "jwt",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );

        // Add global security requirement
        openapi.security = Some(vec![SecurityRequirement::new("jwt", Vec::<String>::new())]);
    }
}
