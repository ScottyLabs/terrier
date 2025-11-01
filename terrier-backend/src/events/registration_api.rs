use actix_web::{web, HttpResponse, Responder};
use sea_orm::DatabaseConnection;
use crate::entities::{user_registered_events, mini_events};
use crate::entities::hackathon_roles::HackathonRole;
use crate::entities::user_hackathon_roles;

pub async fn register_for_event(
    db: web::Data<DatabaseConnection>,
    event_id: web::Path<i32>,
    user: web::ReqData<User>,
) -> impl Responder {
    use sea_orm::{entity::*, query::*};

    // Derive user_role dynamically
    let user_role = user_hackathon_roles::Entity::find()
        .filter(user_hackathon_roles::Column::UserId.eq(user.id))
        .filter(user_hackathon_roles::Column::HackathonId.eq(event_id.into_inner()))
        .one(db.get_ref())
        .await;

    if let Ok(Some(role)) = user_role {
        if !role.is_participant() {
            return HttpResponse::Forbidden().json("Only participants can register for events");
        }
    } else {
        return HttpResponse::Forbidden().json("User role not found or unauthorized");
    }

    // Validate user ID
    let user_exists = crate::entities::users::Entity::find_by_id(user.id)
        .one(db.get_ref())
        .await;

    if user_exists.is_err() || user_exists.unwrap().is_none() {
        return HttpResponse::NotFound().json("User not found");
    }

    let event_id = event_id.into_inner();

    // Check if the event exists
    let event_exists = mini_events::Entity::find_by_id(event_id)
        .one(db.get_ref())
        .await;

    match event_exists {
        Ok(Some(_)) => {
            // Register for the event
            let registration = user_registered_events::ActiveModel {
                user_id: Set(user.id),
                event_id: Set(event_id),
                is_favorite: Set(false),
            };

            match registration.insert(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().json("Successfully registered for the event"),
                Err(err) => HttpResponse::InternalServerError().json(format!("Failed to register: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().json("Event not found"),
        Err(err) => HttpResponse::InternalServerError().json(format!("Database error: {}", err)),
    }
}

pub async fn unregister_from_event(
    db: web::Data<DatabaseConnection>,
    event_id: web::Path<i32>,
    user: web::ReqData<User>,
) -> impl Responder {
    use sea_orm::{entity::*, query::*};

    // Derive user_role dynamically
    let user_role = user_hackathon_roles::Entity::find()
        .filter(user_hackathon_roles::Column::UserId.eq(user.id))
        .filter(user_hackathon_roles::Column::HackathonId.eq(event_id.into_inner()))
        .one(db.get_ref())
        .await;

    if let Ok(Some(role)) = user_role {
        if !role.is_participant() {
            return HttpResponse::Forbidden().json("Only participants can unregister from events");
        }
    } else {
        return HttpResponse::Forbidden().json("User role not found or unauthorized");
    }

    // Validate user ID
    let user_exists = crate::entities::users::Entity::find_by_id(user.id)
        .one(db.get_ref())
        .await;

    if user_exists.is_err() || user_exists.unwrap().is_none() {
        return HttpResponse::NotFound().json("User not found");
    }

    let event_id = event_id.into_inner();

    // Unregister from the event
    let delete_result = user_registered_events::Entity::delete_many()
        .filter(user_registered_events::Column::UserId.eq(user.id))
        .filter(user_registered_events::Column::EventId.eq(event_id))
        .exec(db.get_ref())
        .await;

    match delete_result {
        Ok(delete_result) if delete_result.rows_affected > 0 => {
            HttpResponse::Ok().json("Successfully unregistered from the event")
        }
        Ok(_) => HttpResponse::NotFound().json("Registration not found"),
        Err(err) => HttpResponse::InternalServerError().json(format!("Failed to unregister: {}", err)),
    }
}

pub async fn list_registered_events(
    db: web::Data<DatabaseConnection>,
    user: web::ReqData<User>,
    user_role: HackathonRole,
) -> impl Responder {
    use sea_orm::{entity::*, query::*};

    // Retrieve all registered events for the user
    let registrations = user_registered_events::Entity::find()
        .filter(user_registered_events::Column::UserId.eq(user.id))
        .find_also_related(mini_events::Entity)
        .all(db.get_ref())
        .await;

    match registrations {
        Ok(registrations) => HttpResponse::Ok().json(registrations),
        Err(err) => HttpResponse::InternalServerError().json(format!("Failed to retrieve registered events: {}", err)),
    }
}

pub async fn toggle_favorite_status(
    db: web::Data<DatabaseConnection>,
    event_id: web::Path<i32>,
    user: web::ReqData<User>,
    user_role: HackathonRole,
) -> impl Responder {
    use sea_orm::{entity::*, query::*};

    // Validate user ID
    let user_exists = crate::entities::users::Entity::find_by_id(user.id)
        .one(db.get_ref())
        .await;

    if user_exists.is_err() || user_exists.unwrap().is_none() {
        return HttpResponse::NotFound().json("User not found");
    }

    let event_id = event_id.into_inner();

    // Find the registration
    let registration = user_registered_events::Entity::find()
        .filter(user_registered_events::Column::UserId.eq(user.id))
        .filter(user_registered_events::Column::EventId.eq(event_id))
        .one(db.get_ref())
        .await;

    match registration {
        Ok(Some(mut registration)) => {
            // Toggle the favorite status
            registration.is_favorite = !registration.is_favorite;

            match registration.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().json("Favorite status toggled successfully"),
                Err(err) => HttpResponse::InternalServerError().json(format!("Failed to toggle favorite status: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().json("Registration not found"),
        Err(err) => HttpResponse::InternalServerError().json(format!("Database error: {}", err)),
    }
}

pub async fn get_registered_events(
    db: web::Data<DatabaseConnection>,
    user: web::ReqData<User>,
    user_role: HackathonRole,
) -> impl Responder {
    use sea_orm::{entity::*, query::*};

    // Validate user ID
    let user_exists = crate::entities::users::Entity::find_by_id(user.id)
        .one(db.get_ref())
        .await;

    if user_exists.is_err() || user_exists.unwrap().is_none() {
        return HttpResponse::NotFound().json("User not found");
    }

    // Retrieve all registered events for the user
    let registrations = user_registered_events::Entity::find()
        .filter(user_registered_events::Column::UserId.eq(user.id))
        .find_also_related(mini_events::Entity)
        .all(db.get_ref())
        .await;

    match registrations {
        Ok(registrations) => HttpResponse::Ok().json(registrations),
        Err(err) => HttpResponse::InternalServerError().json(format!("Failed to retrieve registered events: {}", err)),
    }
}

pub async fn get_favorited_events(
    db: web::Data<DatabaseConnection>,
    user: web::ReqData<User>,
    user_role: HackathonRole,
) -> impl Responder {
    use sea_orm::{entity::*, query::*};

    // Validate user ID
    let user_exists = crate::entities::users::Entity::find_by_id(user.id)
        .one(db.get_ref())
        .await;

    if user_exists.is_err() || user_exists.unwrap().is_none() {
        return HttpResponse::NotFound().json("User not found");
    }

    // Retrieve all favorited events for the user
    let favorites = user_registered_events::Entity::find()
        .filter(user_registered_events::Column::UserId.eq(user.id))
        .filter(user_registered_events::Column::IsFavorite.eq(true))
        .find_also_related(mini_events::Entity)
        .all(db.get_ref())
        .await;

    match favorites {
        Ok(favorites) => HttpResponse::Ok().json(favorites),
        Err(err) => HttpResponse::InternalServerError().json(format!("Failed to retrieve favorited events: {}", err)),
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/events/{id}/register")
            .route(web::post().to(register_for_event)),
    );
    cfg.service(
        web::resource("/events/{id}/register")
            .route(web::delete().to(unregister_from_event)),
    );
    cfg.service(
        web::resource("/users/me/registered-events")
            .route(web::get().to(get_registered_events)),
    );
    cfg.service(
        web::resource("/users/me/favorited-events")
            .route(web::get().to(get_favorited_events)),
    );
    cfg.service(
        web::resource("/events/{id}/favorite")
            .route(web::patch().to(toggle_favorite_status)),
    );
}