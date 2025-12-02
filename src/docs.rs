use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        // Hackathons
        crate::hackathons::handlers::create_hackathon::create_hackathon,
        crate::hackathons::handlers::create_hackathon::upload_banner,
        crate::hackathons::handlers::get_hackathons::get_hackathons,
        crate::hackathons::handlers::get_hackathons::get_hackathon_by_slug,
        crate::hackathons::handlers::update_hackathon::update_hackathon,
        crate::hackathons::handlers::update_hackathon::delete_banner,
        // Applications
        crate::hackathons::handlers::applications::update_application,
        crate::hackathons::handlers::applications::submit_application,
        crate::hackathons::handlers::applications::unsubmit_application,
        crate::hackathons::handlers::applications::get_application,
        crate::hackathons::handlers::applications::get_all_applications,
        crate::hackathons::handlers::applications::accept_applications,
        crate::hackathons::handlers::applications::reject_applications,
        crate::hackathons::handlers::applications::decline_attendance,
        crate::hackathons::handlers::applications::confirm_attendance,
        crate::hackathons::handlers::file_upload::upload_application_file,
        crate::hackathons::handlers::file_upload::delete_application_file,
        // People
        crate::hackathons::handlers::people::get_hackathon_people,
        crate::hackathons::handlers::people::remove_hackathon_person,
        // Form Config
        crate::hackathons::handlers::form_config::set_form_config,
        crate::hackathons::handlers::form_config::get_form_config,
        // Teams
        crate::hackathons::handlers::teams::get_my_team,
        crate::hackathons::handlers::teams::get_all_teams,
        crate::hackathons::handlers::teams::update_team,
        crate::hackathons::handlers::teams::join_team,
        crate::hackathons::handlers::teams::leave_team,
        crate::hackathons::handlers::teams::get_team_details,
        // Auth
        crate::auth::handlers::get_current_user,
    ),
    components(
        schemas(
            // Hackathons
            crate::hackathons::HackathonInfo,
            crate::hackathons::handlers::create_hackathon::CreateHackathonRequest,
            crate::hackathons::handlers::update_hackathon::UpdateHackathonRequest,
            // Applications
            crate::hackathons::handlers::applications::ApplicationData,
            crate::hackathons::handlers::applications::ApplicationWithUser,
            crate::hackathons::handlers::file_upload::FileUploadResponse,
            // People
            crate::hackathons::handlers::people::HackathonPerson,
            // Form Config
            crate::schemas::application_form::FormSchema,
            crate::schemas::application_form::FormField,
            crate::schemas::application_form::FieldType,
            crate::schemas::application_form::TextValidation,
            crate::schemas::application_form::NumberValidation,
            crate::schemas::application_form::FileValidation,
            crate::schemas::application_form::SelectOption,
            // Teams
            crate::hackathons::handlers::teams::TeamData,
            crate::hackathons::handlers::teams::TeamMemberData,
            crate::hackathons::handlers::teams::TeamListItem,
            crate::hackathons::handlers::teams::UpdateTeamRequest,
            crate::hackathons::handlers::teams::JoinTeamRequest,
            // Auth
            crate::auth::UserInfo,
        )
    ),
    tags(
        (name = "hackathons", description = "Hackathon management endpoints"),
        (name = "applications", description = "Application management endpoints"),
        (name = "people", description = "People management endpoints"),
        (name = "teams", description = "Team management endpoints"),
        (name = "auth", description = "Authentication endpoints")
    ),
    info(
        title = "Terrier API",
        version = "0.1.0",
        description = "API for Terrier"
    )
)]
pub struct ApiDoc;
