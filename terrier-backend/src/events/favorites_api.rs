use actix_web::{web, HttpResponse, Responder};
use sea_orm::DatabaseConnection;
use crate::entities::{user_favorite_events, mini_events};

pub async fn mark_event_as_favorite(
    db: web::Data<DatabaseConnection>,
    event_id: web::Path<i32>,
    user: web::ReqData<User>,
) -> impl Responder {
    use sea_orm::{entity::*, query::*};

    let event_id = event_id.into_inner();

    // Validate event ID
    let event_exists = mini_events::Entity::find_by_id(event_id)
        .one(db.get_ref())
        .await;

    if event_exists.is_err() || event_exists.unwrap().is_none() {
        return HttpResponse::NotFound().json("Event not found");
    }

    // Add to favorites
    let favorite = user_favorite_events::ActiveModel {
        user_id: Set(user.id),
        event_id: Set(event_id),
    };

    match favorite.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().json("Event marked as favorite"),
        Err(err) => HttpResponse::InternalServerError().json(format!("Failed to mark as favorite: {}", err)),
    }
}

pub async fn unmark_event_as_favorite(
    db: web::Data<DatabaseConnection>,
    event_id: web::Path<i32>,
    user: web::ReqData<User>,
) -> impl Responder {
    use sea_orm::{entity::*, query::*};

    let event_id = event_id.into_inner();

    // Remove from favorites
    let delete_result = user_favorite_events::Entity::delete_many()
        .filter(user_favorite_events::Column::UserId.eq(user.id))
        .filter(user_favorite_events::Column::EventId.eq(event_id))
        .exec(db.get_ref())
        .await;

    match delete_result {
        Ok(delete_result) if delete_result.rows_affected > 0 => {
            HttpResponse::Ok().json("Event unmarked as favorite")
        }
        Ok(_) => HttpResponse::NotFound().json("Favorite not found"),
        Err(err) => HttpResponse::InternalServerError().json(format!("Failed to unmark as favorite: {}", err)),
    }
}

pub async fn list_all_favorite_events(
    db: web::Data<DatabaseConnection>,
    user: web::ReqData<User>,
) -> impl Responder {
    use sea_orm::{entity::*, query::*};

    // Retrieve all favorite events for the user
    let favorites = user_favorite_events::Entity::find()
        .filter(user_favorite_events::Column::UserId.eq(user.id))
        .find_also_related(mini_events::Entity)
        .all(db.get_ref())
        .await;

    match favorites {
        Ok(favorites) => HttpResponse::Ok().json(favorites),
        Err(err) => HttpResponse::InternalServerError().json(format!("Failed to retrieve favorite events: {}", err)),
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/events/{id}/favorite")
            .route(web::post().to(mark_event_as_favorite)),
    );
    cfg.service(
        web::resource("/events/{id}/favorite")
            .route(web::delete().to(unmark_event_as_favorite)),
    );
    cfg.service(
        web::resource("/users/me/favorites")
            .route(web::get().to(list_all_favorite_events)),
    );
}