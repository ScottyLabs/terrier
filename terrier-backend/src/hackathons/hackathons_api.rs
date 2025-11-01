use actix_web::{web, HttpResponse, Responder};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use crate::entities::{hackathons, user_hackathon_roles, users};

#[derive(Deserialize)]
pub struct UpdateHackathonMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<chrono::NaiveDateTime>,
    pub end_date: Option<chrono::NaiveDateTime>,
    pub is_active: Option<bool>,
}

pub async fn update_hackathon_metadata(
    db: web::Data<DatabaseConnection>,
    hackathon_id: web::Path<i32>,
    metadata: web::Json<UpdateHackathonMetadata>,
    user: web::ReqData<User>, // User would be stored in some cookie in frontend
) -> impl Responder {
    use hackathons::Entity as Hackathon;
    use sea_orm::{entity::*, query::*};

    let hackathon_id = hackathon_id.into_inner();
    let metadata = metadata.into_inner();

    // Input validation
    if let Some(start_date) = metadata.start_date {
        if let Some(end_date) = metadata.end_date {
            if start_date > end_date {
                return HttpResponse::BadRequest().json("Start date must be less than or equal to end date");
            }
        }
    }

    // // clanker suggested this but honestly idk
    // if let Some(name) = &metadata.name {
    //     if name.len() > 100 {
    //         return HttpResponse::BadRequest().json("Name must be 100 characters or fewer");
    //     }
    // }

    // Check if the user is an organizer or lead organizer for the hackathon
    let is_authorized = user_hackathon_roles::Entity::find()
        .filter(user_hackathon_roles::Column::HackathonId.eq(hackathon_id))
        .filter(user_hackathon_roles::Column::UserId.eq(user.id))
        .filter(user_hackathon_roles::Column::Role.is_in(vec!["organizer", "lead_organizer"]))
        .one(db.get_ref())
        .await;

    match is_authorized {
        Ok(Some(_)) => {
            // Logging
            log::info!(
                "User {} is updating hackathon {} with metadata: {:?}",
                user.id, hackathon_id, metadata
            );

            // User is authorized, proceed with the update
            let hackathon = Hackathon::find_by_id(hackathon_id)
                .one(db.get_ref())
                .await;

            match hackathon {
                Ok(Some(mut hackathon)) => {
                    // Update fields if provided
                    if let Some(name) = metadata.name {
                        hackathon.name = name;
                    }
                    if let Some(description) = metadata.description {
                        hackathon.description = description;
                    }
                    if let Some(start_date) = metadata.start_date {
                        hackathon.start_date = start_date;
                    }
                    if let Some(end_date) = metadata.end_date {
                        hackathon.end_date = end_date;
                    }
                    if let Some(is_active) = metadata.is_active {
                        hackathon.is_active = is_active;
                    }

                    // Save changes
                    match hackathon.update(db.get_ref()).await {
                        Ok(_) => HttpResponse::Ok().json("Hackathon updated successfully"),
                        Err(err) => HttpResponse::InternalServerError().json(format!("Failed to update hackathon: {}", err)),
                    }
                }
                Ok(None) => HttpResponse::NotFound().json("Hackathon not found"),
                Err(err) => HttpResponse::InternalServerError().json(format!("Database error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::Forbidden().json("You do not have permission to modify this hackathon"),
        Err(err) => HttpResponse::InternalServerError().json(format!("Authorization check failed: {}", err)),
    }
}

pub async fn get_hackathon_metadata(
    db: web::Data<DatabaseConnection>,
    hackathon_id: web::Path<i32>,
) -> impl Responder {
    use hackathons::Entity as Hackathon;
    use sea_orm::entity::*;

    let hackathon_id = hackathon_id.into_inner();

    // Find the hackathon by ID
    match Hackathon::find_by_id(hackathon_id).one(db.get_ref()).await {
        Ok(Some(hackathon)) => HttpResponse::Ok().json(hackathon),
        Ok(None) => HttpResponse::NotFound().json("Hackathon not found"),
        Err(err) => HttpResponse::InternalServerError().json(format!("Database error: {}", err)),
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/hackathons/{id}")
            .route(web::put().to(update_hackathon_metadata))
            .route(web::get().to(get_hackathon_metadata)),
    );
}