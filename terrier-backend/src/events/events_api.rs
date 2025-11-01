use actix_web::{web, HttpResponse, Responder};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use crate::entities::{mini_events, user_hackathon_roles};

#[derive(Deserialize)]
pub struct UpdateEventMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<chrono::NaiveDateTime>,
    pub end_date: Option<chrono::NaiveDateTime>,
    pub location: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Deserialize)]
pub struct NewEvent {
    pub name: String,
    pub description: Option<String>,
    pub start_date: chrono::NaiveDateTime,
    pub end_date: chrono::NaiveDateTime,
    pub location: Option<String>,
    pub is_active: bool,
    pub hackathon_id: i32,
}

pub async fn update_event_metadata(
    db: web::Data<DatabaseConnection>,
    event_id: web::Path<i32>,
    metadata: web::Json<UpdateEventMetadata>,
    user: web::ReqData<User>,
) -> impl Responder {
    use sea_orm::{entity::*, query::*};

    let event_id = event_id.into_inner();
    let metadata = metadata.into_inner();

    // Input validation
    if let Some(start_date) = metadata.start_date {
        if let Some(end_date) = metadata.end_date {
            if start_date >= end_date {
                return HttpResponse::BadRequest().json("Start date must be before end date");
            }
        }
    }

    // Validate event ID
    let event_exists = mini_events::Entity::find_by_id(event_id)
        .one(db.get_ref())
        .await;

    if event_exists.is_err() || event_exists.unwrap().is_none() {
        return HttpResponse::NotFound().json("Event not found");
    }

    // Derive user_role dynamically
    let user_role = user_hackathon_roles::Entity::find()
        .filter(user_hackathon_roles::Column::UserId.eq(user.id))
        .filter(user_hackathon_roles::Column::HackathonId.eq(event_id))
        .one(db.get_ref())
        .await;

    if let Ok(Some(role)) = user_role {
        if !role.is_organizer() {
            return HttpResponse::Forbidden().json("You do not have permission to modify this event");
        }
    } else {
        return HttpResponse::Forbidden().json("User role not found or unauthorized");
    }

    // Check if the user is an organizer or lead organizer for the event's hackathon
    let is_authorized = user_hackathon_roles::Entity::find()
        .filter(user_hackathon_roles::Column::UserId.eq(user.id))
        .filter(user_hackathon_roles::Column::Role.is_in(vec!["organizer", "lead_organizer"]))
        .join(JoinType::InnerJoin, user_hackathon_roles::Relation::Hackathon.def())
        .join(JoinType::InnerJoin, mini_events::Relation::Hackathon.def())
        .filter(mini_events::Column::Id.eq(event_id))
        .one(db.get_ref())
        .await;

    match is_authorized {
        Ok(Some(_)) => {
            // User is authorized, proceed with the update
            let event = mini_events::Entity::find_by_id(event_id)
                .one(db.get_ref())
                .await;

            match event {
                Ok(Some(mut event)) => {
                    // Update fields if provided
                    if let Some(name) = metadata.name {
                        event.name = name;
                    }
                    if let Some(description) = metadata.description {
                        event.description = description;
                    }
                    if let Some(start_date) = metadata.start_date {
                        event.start_date = start_date;
                    }
                    if let Some(end_date) = metadata.end_date {
                        event.end_date = end_date;
                    }
                    if let Some(location) = metadata.location {
                        event.location = Some(location);
                    }
                    if let Some(is_active) = metadata.is_active {
                        event.is_active = is_active;
                    }

                    // Save changes
                    match event.update(db.get_ref()).await {
                        Ok(_) => HttpResponse::Ok().json("Event updated successfully"),
                        Err(err) => HttpResponse::InternalServerError().json(format!("Failed to update event: {}", err)),
                    }
                }
                Ok(None) => HttpResponse::NotFound().json("Event not found"),
                Err(err) => HttpResponse::InternalServerError().json(format!("Database error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::Forbidden().json("You do not have permission to modify this event"),
        Err(err) => HttpResponse::InternalServerError().json(format!("Authorization check failed: {}", err)),
    }
}

pub async fn create_event(
    db: web::Data<DatabaseConnection>,
    new_event: web::Json<NewEvent>,
    user: web::ReqData<User>,
) -> impl Responder {
    use sea_orm::{entity::*, query::*};

    // Check if the user is an organizer for the hackathon
    let user_role = user_hackathon_roles::Entity::find()
        .filter(user_hackathon_roles::Column::UserId.eq(user.id))
        .filter(user_hackathon_roles::Column::HackathonId.eq(new_event.hackathon_id))
        .one(db.get_ref())
        .await;

    if let Ok(Some(role)) = user_role {
        if !role.is_organizer() {
            return HttpResponse::Forbidden().json("Only organizers can create events");
        }
    } else {
        return HttpResponse::Forbidden().json("User role not found or unauthorized");
    }

    // Create the new event
    let new_event = new_event.into_inner();
    let event = mini_events::ActiveModel {
        name: Set(new_event.name),
        description: Set(new_event.description),
        start_date: Set(new_event.start_date),
        end_date: Set(new_event.end_date),
        location: Set(new_event.location),
        is_active: Set(new_event.is_active),
        hackathon_id: Set(new_event.hackathon_id),
        ..Default::default()
    };

    match event.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Created().json("Event created successfully"),
        Err(err) => HttpResponse::InternalServerError().json(format!("Failed to create event: {}", err)),
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/events/{id}")
            .route(web::put().to(update_event_metadata)),
    );
    cfg.service(
        web::resource("/events")
            .route(web::post().to(create_event)),
    );
}