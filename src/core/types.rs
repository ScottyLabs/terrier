// Re-export commonly used types from entities for convenience
pub use crate::entities::{
    applications::Model as ApplicationEntity, hackathons::Model as HackathonEntity,
    teams::Model as TeamEntity, user_hackathon_roles::Model as UserHackathonRoleEntity,
    users::Model as UserEntity,
};
