mod common;

use std::sync::atomic::Ordering;

use axum::{
    body::{Body, to_bytes},
    http::{
        Method, Request, StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS},
    },
};
use common::{TestApp, TestSession};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use terrier_server::{audit::Scope, error::ApiError, files::remove_unused_uploads};
use uuid::Uuid;

const PDF: &[u8] = b"%PDF-1.4\n1 0 obj <</Type/Catalog/Pages 2 0 R>> endobj\n2 0 obj <</Type/Pages/Kids[]/Count 0>> endobj\ntrailer <</Root 1 0 R>>\n%%EOF\n";
const SCRIPT_PDF: &[u8] = b"%PDF-1.4\n1 0 obj <</Type/Catalog/Pages 2 0 R/OpenAction<</S/JavaScript/JS(app.alert(1))>>>> endobj\n2 0 obj <</Type/Pages/Kids[]/Count 0>> endobj\ntrailer <</Root 1 0 R>>\n%%EOF\n";
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";

async fn fixture(app: &TestApp, scope: Scope, sql: String) -> anyhow::Result<()> {
    terrier_server::audit::automatic(&app.db, scope, "Integration test fixture", |tx| {
        Box::pin(async move {
            tx.execute_unprepared(&sql)
                .await
                .map_err(|error| ApiError::Internal(error.into()))?;
            Ok(())
        })
    })
    .await
    .map_err(|error| anyhow::anyhow!("{error:?}"))
}

/// Creates an open hackathon with the default file fields.
async fn hackathon(app: &TestApp, slug: &str) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    fixture(app, Scope::Global, format!(
        "INSERT INTO hackathon(id,slug,name,description,is_active,max_team_size,max_participants,location,contact_email,application_schema,data,application_deadline,team_formation_deadline,submission_deadline)
            VALUES('{id}','{slug}','Fixture','Fixture',true,4,100,'Campus','fixture@example.com','{{}}','{{}}',clock_timestamp()+interval '1 day',clock_timestamp()+interval '2 days',clock_timestamp()+interval '3 days')"
    )).await?;
    Ok(id)
}

async fn json_request(
    app: &TestApp,
    session: &TestSession,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> anyhow::Result<(StatusCode, Value)> {
    let request = session.authorize(Request::builder().method(method).uri(path));
    let request = match body {
        Some(body) => request
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))?,
        None => request.body(Body::empty())?,
    };
    let response = app.send(request).await?;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1 << 20).await?;
    Ok((status, serde_json::from_slice(&bytes)?))
}

async fn upload(
    app: &TestApp,
    session: &TestSession,
    slug: &str,
    applicant: Uuid,
    field: &str,
    filename: &str,
    contents: &[u8],
) -> anyhow::Result<(StatusCode, Value)> {
    let boundary = "terrier-test-boundary";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(contents);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let response = app
        .send(
            session
                .authorize(Request::post(format!(
                    "/api/hackathons/{slug}/applications/{applicant}/files?field={field}"
                )))
                .header(
                    CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))?,
        )
        .await?;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1 << 20).await?;
    Ok((status, serde_json::from_slice(&bytes)?))
}

async fn download(
    app: &TestApp,
    session: &TestSession,
    path: &str,
) -> anyhow::Result<axum::response::Response> {
    app.send(session.authorize(Request::get(path)).body(Body::empty())?)
        .await
}

fn file_ids(application: &Value) -> Vec<&str> {
    application["files"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|file| file["id"].as_str())
        .collect()
}

#[tokio::test]
async fn applicant_uploads_replaces_and_submits_a_resume() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let hacker = app.login("hacker").await?;
    hackathon(&app, "tartanhacks").await?;
    let application = format!(
        "/api/hackathons/tartanhacks/applications/{}",
        hacker.user_id
    );

    let (status, body) = upload(
        &app,
        &hacker,
        "tartanhacks",
        hacker.user_id,
        "resume",
        "resume.pdf",
        PDF,
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");

    let (status, _) = json_request(
        &app,
        &hacker,
        Method::PUT,
        &application,
        Some(json!({"data": {}})),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);

    for (field, filename, contents, error) in [
        ("resume", "photo.pdf", PNG, "file"),
        ("resume", "script.pdf", SCRIPT_PDF, "file"),
        ("resume", "notes.pdf", b"plain text".as_slice(), "file"),
        ("resume", "empty.pdf", b"".as_slice(), "file"),
        ("portfolio", "resume.pdf", PDF, "field"),
    ] {
        let (status, body) = upload(
            &app,
            &hacker,
            "tartanhacks",
            hacker.user_id,
            field,
            filename,
            contents,
        )
        .await?;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{filename}: {body}"
        );
        assert!(body["fields"][error].is_string(), "{body}");
    }
    assert!(app.storage.objects.lock().unwrap().is_empty());

    let (status, first) = upload(
        &app,
        &hacker,
        "tartanhacks",
        hacker.user_id,
        "resume",
        "../My Resume.pdf",
        PDF,
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    assert_eq!(first["filename"], "My Resume.pdf");
    assert_eq!(first["content_type"], "application/pdf");
    assert_eq!(first["size"], PDF.len());
    assert_eq!(first["submitted"], false);
    let first_id = first["id"].as_str().unwrap().to_owned();

    let (status, body) = json_request(
        &app,
        &hacker,
        Method::PUT,
        &application,
        Some(json!({"data": {"resume": first_id}, "submit": true})),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(file_ids(&body), [first_id.as_str()]);
    assert_eq!(body["files"][0]["submitted"], true);

    let (status, second) = upload(
        &app,
        &hacker,
        "tartanhacks",
        hacker.user_id,
        "resume",
        "resume-v2.pdf",
        PDF,
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED);
    let second_id = second["id"].as_str().unwrap().to_owned();
    let (status, body) = json_request(
        &app,
        &hacker,
        Method::PUT,
        &application,
        Some(json!({"data": {"resume": second_id}})),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["resume"], first_id);
    assert_eq!(body["draft_data"]["resume"], second_id);
    assert_eq!(body["files"][0]["submitted"], true);
    assert_eq!(body["files"][1]["submitted"], false);
    assert_eq!(app.storage.objects.lock().unwrap().len(), 2);

    let snapshot: String = app
        .db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT e.after_data->'data'->>'resume' AS resume FROM application a
                JOIN audit_event e ON e.id=a.submitted_audit_id WHERE a.user_id=$1",
            [hacker.user_id.into()],
        ))
        .await?
        .ok_or_else(|| anyhow::anyhow!("missing submission snapshot"))?
        .try_get("", "resume")?;
    assert_eq!(snapshot, first_id);

    let response = download(&app, &hacker, &format!("{application}/files/{first_id}")).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CONTENT_TYPE], "application/pdf");
    assert_eq!(
        response.headers()[CONTENT_DISPOSITION],
        "attachment; filename=\"My Resume.pdf\""
    );
    assert_eq!(response.headers()[X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert_eq!(to_bytes(response.into_body(), 1 << 20).await?, PDF);

    app.close().await
}

#[tokio::test]
async fn files_are_visible_to_their_owner_and_submitted_files_to_managers() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let hacker = app.login("hacker").await?;
    let other = app.login("other-hacker").await?;
    let manager = app.login("manager").await?;
    let organizer = app.login("organizer").await?;
    let id = hackathon(&app, "tartanhacks").await?;
    for (user, capabilities) in [
        (manager.user_id, "'application_management'"),
        (organizer.user_id, ""),
    ] {
        fixture(&app, Scope::HackathonExclusive(id), format!(
            "INSERT INTO user_hackathon_role(user_id,hackathon_id,role) VALUES('{user}','{id}','organizer');
             INSERT INTO organizer(user_id,hackathon_id,capabilities) VALUES('{user}','{id}',ARRAY[{capabilities}]::capability[])"
        )).await?;
    }
    let application = format!(
        "/api/hackathons/tartanhacks/applications/{}",
        hacker.user_id
    );
    json_request(
        &app,
        &hacker,
        Method::PUT,
        &application,
        Some(json!({"data": {}})),
    )
    .await?;
    let mut ids = Vec::new();
    for name in ["submitted.pdf", "draft.pdf"] {
        let (status, file) = upload(
            &app,
            &hacker,
            "tartanhacks",
            hacker.user_id,
            "resume",
            name,
            PDF,
        )
        .await?;
        assert_eq!(status, StatusCode::CREATED);
        ids.push(file["id"].as_str().unwrap().to_owned());
    }
    let (submitted, draft) = (&ids[0], &ids[1]);
    json_request(
        &app,
        &hacker,
        Method::PUT,
        &application,
        Some(json!({"data": {"resume": submitted}, "submit": true})),
    )
    .await?;

    let (status, body) = json_request(&app, &manager, Method::GET, &application, None).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(file_ids(&body), [submitted.as_str()]);

    for (session, file, expected) in [
        (&hacker, submitted, StatusCode::OK),
        (&hacker, draft, StatusCode::OK),
        (&manager, submitted, StatusCode::OK),
        (&manager, draft, StatusCode::NOT_FOUND),
        (&organizer, submitted, StatusCode::FORBIDDEN),
        (&other, submitted, StatusCode::FORBIDDEN),
    ] {
        let response = download(&app, session, &format!("{application}/files/{file}")).await?;
        assert_eq!(response.status(), expected, "{file}");
    }
    let missing = Uuid::new_v4();
    let response = download(&app, &hacker, &format!("{application}/files/{missing}")).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = download(&app, &hacker, &format!("{application}/files/not-a-uuid")).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let (status, _) = upload(
        &app,
        &other,
        "tartanhacks",
        hacker.user_id,
        "resume",
        "resume.pdf",
        PDF,
    )
    .await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let other_application = format!("/api/hackathons/tartanhacks/applications/{}", other.user_id);
    let (status, body) = json_request(
        &app,
        &other,
        Method::PUT,
        &other_application,
        Some(json!({"data": {"resume": submitted}})),
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["fields"]["data.resume"].is_string());

    app.close().await
}

#[tokio::test]
async fn hackathon_file_fields_set_types_and_sizes() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let hacker = app.login("hacker").await?;
    let id = hackathon(&app, "tartanhacks").await?;
    fixture(
        &app,
        Scope::HackathonExclusive(id),
        format!(
            r#"UPDATE hackathon SET file_fields='{{"portfolio":{{"types":["*/*"],"max_bytes":16}}}}' WHERE id='{id}'"#
        ),
    )
    .await?;
    let application = format!(
        "/api/hackathons/tartanhacks/applications/{}",
        hacker.user_id
    );
    json_request(
        &app,
        &hacker,
        Method::PUT,
        &application,
        Some(json!({"data": {}})),
    )
    .await?;

    let (status, body) = upload(
        &app,
        &hacker,
        "tartanhacks",
        hacker.user_id,
        "resume",
        "resume.pdf",
        PDF,
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["fields"]["field"].is_string());

    let (status, body) = upload(
        &app,
        &hacker,
        "tartanhacks",
        hacker.user_id,
        "portfolio",
        "too-big.txt",
        &[b'a'; 17],
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["fields"]["file"].is_string());

    let (status, body) = upload(
        &app,
        &hacker,
        "tartanhacks",
        hacker.user_id,
        "portfolio",
        "notes.txt",
        b"plain text",
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["content_type"], "application/octet-stream");

    app.close().await
}

#[tokio::test]
async fn uploads_respect_deadlines_and_storage_failures() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let hacker = app.login("hacker").await?;
    let id = hackathon(&app, "tartanhacks").await?;
    let application = format!(
        "/api/hackathons/tartanhacks/applications/{}",
        hacker.user_id
    );
    json_request(
        &app,
        &hacker,
        Method::PUT,
        &application,
        Some(json!({"data": {}})),
    )
    .await?;

    app.storage.fail.store(true, Ordering::SeqCst);
    let (status, _) = upload(
        &app,
        &hacker,
        "tartanhacks",
        hacker.user_id,
        "resume",
        "resume.pdf",
        PDF,
    )
    .await?;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    app.storage.fail.store(false, Ordering::SeqCst);
    let (_, body) = json_request(&app, &hacker, Method::GET, &application, None).await?;
    assert!(file_ids(&body).is_empty());

    fixture(
        &app,
        Scope::HackathonExclusive(id),
        format!("UPDATE hackathon SET application_deadline=clock_timestamp()-interval '1 minute' WHERE id='{id}'"),
    )
    .await?;
    let (status, _) = upload(
        &app,
        &hacker,
        "tartanhacks",
        hacker.user_id,
        "resume",
        "resume.pdf",
        PDF,
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(app.storage.objects.lock().unwrap().is_empty());

    app.close().await
}

async fn raw_upload(
    app: &TestApp,
    session: Option<&TestSession>,
    uri: &str,
    content_type: &str,
    body: Vec<u8>,
) -> anyhow::Result<(StatusCode, Value)> {
    let mut request = Request::post(uri);
    if let Some(session) = session {
        request = session.authorize(request);
    }
    let response = app
        .send(
            request
                .header(CONTENT_TYPE, content_type)
                .body(Body::from(body))?,
        )
        .await?;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1 << 20).await?;
    Ok((status, serde_json::from_slice(&bytes)?))
}

#[tokio::test]
async fn malformed_and_signed_out_requests_are_rejected() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let hacker = app.login("hacker").await?;
    hackathon(&app, "tartanhacks").await?;
    let application = format!(
        "/api/hackathons/tartanhacks/applications/{}",
        hacker.user_id
    );
    json_request(
        &app,
        &hacker,
        Method::PUT,
        &application,
        Some(json!({"data": {}})),
    )
    .await?;
    let multipart = "multipart/form-data; boundary=b";
    let other_part =
        b"--b\r\nContent-Disposition: form-data; name=\"other\"\r\n\r\nvalue\r\n--b--\r\n";

    let (status, _) = raw_upload(
        &app,
        None,
        &format!("{application}/files?field=resume"),
        multipart,
        other_part.to_vec(),
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    for (uri, content_type, expected, field) in [
        (
            format!("{application}/files"),
            multipart,
            StatusCode::BAD_REQUEST,
            None,
        ),
        (
            format!("{application}/files?field=resume"),
            "application/json",
            StatusCode::BAD_REQUEST,
            None,
        ),
        (
            format!("{application}/files?field=resume"),
            multipart,
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("file"),
        ),
    ] {
        let (status, body) =
            raw_upload(&app, Some(&hacker), &uri, content_type, other_part.to_vec()).await?;
        assert_eq!(status, expected, "{uri} {content_type}: {body}");
        if let Some(field) = field {
            assert!(body["fields"][field].is_string(), "{body}");
        }
    }

    let (status, file) = upload(
        &app,
        &hacker,
        "tartanhacks",
        hacker.user_id,
        "resume",
        "resume.pdf",
        PDF,
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED);
    let response = app
        .send(
            Request::get(format!(
                "{application}/files/{}",
                file["id"].as_str().unwrap()
            ))
            .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let (status, body) = json_request(
        &app,
        &hacker,
        Method::PUT,
        &application,
        Some(json!({"data": {"resume": null}})),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");

    app.close().await
}

#[tokio::test]
async fn decided_or_deactivated_applications_reject_uploads() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let accepted = app.login("accepted").await?;
    let deactivated = app.login("deactivated").await?;
    let id = hackathon(&app, "tartanhacks").await?;
    for hacker in [&accepted, &deactivated] {
        let application = format!(
            "/api/hackathons/tartanhacks/applications/{}",
            hacker.user_id
        );
        let (status, _) = json_request(
            &app,
            hacker,
            Method::PUT,
            &application,
            Some(json!({"data": {}, "submit": true})),
        )
        .await?;
        assert_eq!(status, StatusCode::OK);
    }
    for sql in [
        format!(
            "UPDATE application SET status='accepted' WHERE user_id='{}'",
            accepted.user_id
        ),
        format!(
            "UPDATE application SET is_active=false WHERE user_id='{}'",
            deactivated.user_id
        ),
    ] {
        fixture(&app, Scope::Hackathon(id), sql).await?;
    }

    for hacker in [&accepted, &deactivated] {
        let (status, body) = upload(
            &app,
            hacker,
            "tartanhacks",
            hacker.user_id,
            "resume",
            "resume.pdf",
            PDF,
        )
        .await?;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
    }
    assert!(app.storage.objects.lock().unwrap().is_empty());

    app.close().await
}

#[tokio::test]
async fn downloads_stay_in_their_hackathon_and_report_storage_failures() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let hacker = app.login("hacker").await?;
    let outsider = app.login("other-manager").await?;
    hackathon(&app, "tartanhacks").await?;
    let other = hackathon(&app, "otherhacks").await?;
    fixture(&app, Scope::HackathonExclusive(other), format!(
        "INSERT INTO user_hackathon_role(user_id,hackathon_id,role) VALUES('{user}','{other}','organizer');
         INSERT INTO organizer(user_id,hackathon_id,capabilities) VALUES('{user}','{other}',ARRAY['application_management']::capability[])",
        user = outsider.user_id
    )).await?;
    let application = format!(
        "/api/hackathons/tartanhacks/applications/{}",
        hacker.user_id
    );
    json_request(
        &app,
        &hacker,
        Method::PUT,
        &application,
        Some(json!({"data": {}})),
    )
    .await?;
    let (_, file) = upload(
        &app,
        &hacker,
        "tartanhacks",
        hacker.user_id,
        "resume",
        "resume.pdf",
        PDF,
    )
    .await?;
    let id = file["id"].as_str().unwrap();
    json_request(
        &app,
        &hacker,
        Method::PUT,
        &application,
        Some(json!({"data": {"resume": id}, "submit": true})),
    )
    .await?;
    let path = format!("{application}/files/{id}");

    let response = download(&app, &outsider, &path).await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let other_path = format!(
        "/api/hackathons/otherhacks/applications/{}/files/{id}",
        hacker.user_id
    );
    let response = download(&app, &outsider, &other_path).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    app.storage.fail.store(true, Ordering::SeqCst);
    let response = download(&app, &hacker, &path).await?;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    app.storage.fail.store(false, Ordering::SeqCst);
    let response = download(&app, &hacker, &path).await?;
    assert_eq!(response.status(), StatusCode::OK);

    app.close().await
}

#[tokio::test]
async fn uploads_stop_at_the_server_size_cap() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let hacker = app.login("hacker").await?;
    let id = hackathon(&app, "tartanhacks").await?;
    let cap = terrier_server::files::MAX_FILE_BYTES;
    fixture(
        &app,
        Scope::HackathonExclusive(id),
        format!(
            r#"UPDATE hackathon SET file_fields='{{"portfolio":{{"types":["*/*"],"max_bytes":{cap}}}}}' WHERE id='{id}'"#
        ),
    )
    .await?;
    let application = format!(
        "/api/hackathons/tartanhacks/applications/{}",
        hacker.user_id
    );
    json_request(
        &app,
        &hacker,
        Method::PUT,
        &application,
        Some(json!({"data": {}})),
    )
    .await?;

    for size in [cap + 1, cap + 256 * 1024] {
        let (status, body) = upload(
            &app,
            &hacker,
            "tartanhacks",
            hacker.user_id,
            "portfolio",
            "large.bin",
            &vec![b'a'; size as usize],
        )
        .await?;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{size}: {body}");
        assert!(body["fields"]["file"].is_string(), "{body}");
    }
    let (status, _) = upload(
        &app,
        &hacker,
        "tartanhacks",
        hacker.user_id,
        "portfolio",
        "exact.bin",
        &vec![b'a'; cap as usize],
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(app.storage.objects.lock().unwrap().len(), 1);

    app.close().await
}

async fn age_uploads(app: &TestApp, hackathon_id: Uuid, ids: &[&str]) -> anyhow::Result<()> {
    let ids = ids
        .iter()
        .map(|id| format!("'{id}'"))
        .collect::<Vec<_>>()
        .join(",");
    fixture(
        app,
        Scope::Hackathon(hackathon_id),
        format!(
            "UPDATE application SET attachments=(SELECT jsonb_agg(CASE WHEN f->>'id' IN ({ids})
                THEN jsonb_set(f,'{{uploaded_at}}',to_jsonb(clock_timestamp()-interval '2 days'))
                ELSE f END ORDER BY n) FROM jsonb_array_elements(attachments) WITH ORDINALITY t(f,n))
                WHERE hackathon_id='{hackathon_id}' AND jsonb_array_length(attachments)>0"
        ),
    )
    .await
}

#[tokio::test]
async fn cleanup_removes_only_old_unused_uploads() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let hacker = app.login("hacker").await?;
    let hackathon_id = hackathon(&app, "tartanhacks").await?;
    let application = format!(
        "/api/hackathons/tartanhacks/applications/{}",
        hacker.user_id
    );
    let (status, _) = json_request(
        &app,
        &hacker,
        Method::PUT,
        &application,
        Some(json!({"data": {}})),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);

    let mut ids = Vec::new();
    for name in ["submitted.pdf", "draft.pdf", "old.pdf", "recent.pdf"] {
        let (status, body) = upload(
            &app,
            &hacker,
            "tartanhacks",
            hacker.user_id,
            "resume",
            name,
            PDF,
        )
        .await?;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        ids.push(body["id"].as_str().unwrap().to_owned());
    }
    let [submitted, draft, old, recent] = [&ids[0], &ids[1], &ids[2], &ids[3]];
    for body in [
        json!({"data": {"resume": submitted}, "submit": true}),
        json!({"data": {"resume": draft.to_uppercase()}}),
    ] {
        let (status, body) =
            json_request(&app, &hacker, Method::PUT, &application, Some(body)).await?;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
    age_uploads(&app, hackathon_id, &[submitted, draft, old]).await?;

    app.storage.fail.store(true, Ordering::SeqCst);
    assert_eq!(
        remove_unused_uploads(&app.db, &app.bucket)
            .await
            .map_err(|error| anyhow::anyhow!("{error:?}"))?,
        0
    );
    app.storage.fail.store(false, Ordering::SeqCst);
    assert_eq!(app.storage.objects.lock().unwrap().len(), 4);

    for expected in [1, 0] {
        assert_eq!(
            remove_unused_uploads(&app.db, &app.bucket)
                .await
                .map_err(|error| anyhow::anyhow!("{error:?}"))?,
            expected
        );
    }
    assert_eq!(app.storage.objects.lock().unwrap().len(), 3);

    let (status, body) = json_request(&app, &hacker, Method::GET, &application, None).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        file_ids(&body),
        [submitted.as_str(), draft.as_str(), recent.as_str()]
    );
    let response = download(&app, &hacker, &format!("{application}/files/{old}")).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = download(&app, &hacker, &format!("{application}/files/{draft}")).await?;
    assert_eq!(response.status(), StatusCode::OK);

    for expected in [StatusCode::CREATED; 8]
        .into_iter()
        .chain([StatusCode::CONFLICT])
    {
        let (status, body) = upload(
            &app,
            &hacker,
            "tartanhacks",
            hacker.user_id,
            "resume",
            "extra.pdf",
            PDF,
        )
        .await?;
        assert_eq!(status, expected, "{body}");
    }
    assert_eq!(app.storage.objects.lock().unwrap().len(), 11);

    app.close().await
}
