use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        for function in FUNCTIONS {
            db.execute_unprepared(&function.create()).await?;
        }

        db.execute_unprepared(
            "CREATE TRIGGER audit_event_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON audit_event
    FOR EACH STATEMENT EXECUTE FUNCTION reject_audit_changes()",
        )
        .await?;
        db.execute_unprepared("REVOKE ALL ON audit_event FROM PUBLIC")
            .await?;

        for (name, table, function) in [
            (
                "application_snapshot",
                "application",
                "validate_audit_snapshot",
            ),
            (
                "evaluation_snapshot",
                "evaluation",
                "validate_audit_snapshot",
            ),
            ("team_roster", "team_member", "validate_team_roster"),
            ("team_roster", "team", "validate_team_roster"),
        ] {
            db.execute_unprepared(&format!(
                "CREATE CONSTRAINT TRIGGER {name} AFTER INSERT OR UPDATE ON \"{table}\"
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION {function}()"
            ))
            .await?;
        }

        for table in [
            "hackathon",
            "invitation",
            "reimbursement_application",
            "prize_track",
        ] {
            db.execute_unprepared(&format!(
                "CREATE TRIGGER validate_scoped_json BEFORE INSERT OR UPDATE ON \"{table}\" FOR EACH ROW EXECUTE FUNCTION validate_scoped_json()"
            ))
            .await?;
        }

        db.execute_unprepared(
            "CREATE TRIGGER snapshot_project_tracks AFTER INSERT OR UPDATE ON project_track_sub
    FOR EACH ROW EXECUTE FUNCTION snapshot_project_tracks()",
        )
        .await?;

        for (table, keys) in AUDITED {
            let keys = keys
                .iter()
                .map(|key| format!("'{key}'"))
                .collect::<Vec<_>>()
                .join(",");
            db.execute_unprepared(&format!(
                "CREATE TRIGGER preserve_domain BEFORE TRUNCATE ON \"{table}\" FOR EACH STATEMENT EXECUTE FUNCTION reject_domain_truncate()"
            ))
            .await?;
            db.execute_unprepared(&format!(
                "CREATE TRIGGER prepare_change BEFORE INSERT OR UPDATE OR DELETE ON \"{table}\" FOR EACH ROW EXECUTE FUNCTION prepare_domain_change()"
            ))
            .await?;
            db.execute_unprepared(&format!(
                "CREATE TRIGGER audit_change AFTER INSERT OR UPDATE ON \"{table}\" FOR EACH ROW EXECUTE FUNCTION audit_domain_change({keys})"
            ))
            .await?;
        }

        // the functions run with a fixed search path, so callers can't shadow the tables they use
        let query = Query::select()
            .expr_as(
                Func::cust(Alias::new("quote_ident")).arg(Func::cust(Alias::new("current_schema"))),
                Alias::new("schema"),
            )
            .to_owned();
        let schema: String = db
            .query_one(&query)
            .await?
            .ok_or_else(|| DbErr::Custom("current_schema() returned no row".into()))?
            .try_get("", "schema")?;
        for function in FUNCTIONS {
            db.execute_unprepared(&format!(
                "ALTER FUNCTION {}() SET search_path TO pg_catalog, {schema}, pg_temp",
                function.name
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, _: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "Audit history is retained; this migration is forward-only".into(),
        ))
    }
}

/// Domain tables and the columns that identify a row in its audit history
const AUDITED: [(&str, &[&str]); 18] = [
    ("user", &["id"]),
    ("hackathon", &["id"]),
    ("invitation", &["id"]),
    ("user_hackathon_role", &["user_id", "hackathon_id"]),
    ("application", &["id"]),
    ("reimbursement_application", &["id"]),
    ("team", &["id"]),
    ("team_member", &["team_id", "user_id"]),
    ("project", &["id"]),
    ("sponsor_org", &["id"]),
    ("prize_track", &["id"]),
    ("project_track_sub", &["project_id", "track_id"]),
    ("judge_track_assign", &["user_id", "track_id"]),
    ("event", &["id"]),
    ("event_check_in", &["id"]),
    ("evaluation", &["id"]),
    ("sponsor", &["user_id", "hackathon_id", "org_id"]),
    ("organizer", &["user_id", "hackathon_id"]),
];

/// A PL/pgSQL trigger function
struct Function {
    name: &'static str,
    security_definer: bool,
    body: &'static str,
}

impl Function {
    fn create(&self) -> String {
        let security = if self.security_definer {
            " SECURITY DEFINER"
        } else {
            ""
        };
        format!(
            "CREATE FUNCTION {}() RETURNS trigger LANGUAGE plpgsql{security} AS $${}$$",
            self.name, self.body
        )
    }
}

const FUNCTIONS: [Function; 8] = [
    Function {
        name: "prepare_domain_change",
        security_definer: false,
        body: r#"
DECLARE event_id uuid := gen_random_uuid();
BEGIN
    IF TG_OP='DELETE' THEN
        RAISE EXCEPTION 'Domain rows must be deactivated, not deleted' USING ERRCODE='42501';
    END IF;
    IF TG_OP='INSERT' THEN
        NEW.version := 1;
    ELSE
        NEW.version := OLD.version + 1;
    END IF;
    IF TG_TABLE_NAME='application' THEN
      IF NEW.status IS NOT NULL THEN
        IF TG_OP='INSERT' THEN
            NEW.submitted_audit_id := event_id;
        ELSIF NEW.data IS DISTINCT FROM OLD.data OR NEW.submitted_at IS DISTINCT FROM OLD.submitted_at THEN
            NEW.submitted_audit_id := event_id;
        END IF;
      END IF;
    END IF;
    RETURN NEW;
END "#,
    },
    Function {
        name: "reject_domain_truncate",
        security_definer: false,
        body: r#"
BEGIN
    RAISE EXCEPTION 'Domain rows cannot be truncated; deactivate records instead' USING ERRCODE='42501';
END "#,
    },
    Function {
        name: "reject_audit_changes",
        security_definer: false,
        body: r#"
BEGIN
    RAISE EXCEPTION 'Audit history is append-only' USING ERRCODE='42501';
END "#,
    },
    Function {
        name: "audit_domain_change",
        security_definer: true,
        body: r#"
DECLARE
    actor uuid;
    explanation text;
    operation audit_action;
    previous jsonb;
    current_data jsonb;
    resource_identity jsonb := '{}';
    old_key jsonb := '{}';
    scope uuid;
    event_id uuid := gen_random_uuid();
    key_column text;
BEGIN
    IF TG_OP='DELETE' THEN
        RAISE EXCEPTION 'Domain rows must be deactivated, not deleted' USING ERRCODE='42501';
    END IF;
    actor := nullif(current_setting('terrier.actor_user_id',true),'')::uuid;
    explanation := nullif(btrim(current_setting('terrier.audit_reason',true)),'');
    IF actor IS NULL AND (current_setting('terrier.automatic',true) IS DISTINCT FROM 'true' OR explanation IS NULL) THEN
        RAISE EXCEPTION 'Domain writes require an audited transaction' USING ERRCODE='42501';
    END IF;
    operation := CASE WHEN TG_OP='INSERT' THEN 'created'::audit_action ELSE 'updated'::audit_action END;
    IF TG_OP='UPDATE' THEN previous := to_jsonb(OLD); END IF;
    IF TG_TABLE_NAME='application' THEN
        IF TG_OP='INSERT' THEN
            event_id := coalesce(NEW.submitted_audit_id,event_id);
        ELSIF NEW.submitted_audit_id IS DISTINCT FROM OLD.submitted_audit_id THEN
            event_id := coalesce(NEW.submitted_audit_id,event_id);
        END IF;
    END IF;
    current_data := to_jsonb(NEW);
    FOREACH key_column IN ARRAY TG_ARGV LOOP
        resource_identity := resource_identity || jsonb_build_object(key_column,current_data->key_column);
        IF TG_OP='UPDATE' THEN
            old_key := old_key || jsonb_build_object(key_column,previous->key_column);
        END IF;
    END LOOP;
    IF TG_OP='UPDATE' AND old_key <> resource_identity THEN
        RAISE EXCEPTION 'Resource identity cannot be changed' USING ERRCODE='23514';
    END IF;
    scope := CASE WHEN TG_TABLE_NAME='hackathon' THEN (current_data->>'id')::uuid
        WHEN current_data ? 'hackathon_id' THEN (current_data->>'hackathon_id')::uuid
        WHEN TG_TABLE_NAME='event_check_in' THEN (SELECT hackathon_id FROM event WHERE id=(current_data->>'event_id')::uuid)
        ELSE NULL END;
    IF scope IS NOT NULL AND (TG_TABLE_NAME <> 'hackathon' OR TG_OP='UPDATE') AND
        nullif(current_setting('terrier.hackathon_id',true),'')::uuid IS DISTINCT FROM scope THEN
        RAISE EXCEPTION 'Write does not match the audited hackathon scope' USING ERRCODE='42501';
    END IF;
    IF (TG_TABLE_NAME IN ('organizer','sponsor','invitation','user_hackathon_role')
        OR (TG_TABLE_NAME='hackathon' AND TG_OP='UPDATE')
        OR (TG_TABLE_NAME='judge_track_assign' AND (TG_OP='INSERT'
            OR current_data->'manually_granted' IS DISTINCT FROM previous->'manually_granted'
            OR current_data->'sponsor_granted' IS DISTINCT FROM previous->'sponsor_granted'))
        OR (TG_TABLE_NAME='team' AND (current_data->>'table_number' IS NOT NULL)
            AND (TG_OP='INSERT' OR current_data->'table_number' IS DISTINCT FROM previous->'table_number')))
        AND current_setting('terrier.management',true) IS DISTINCT FROM 'true' THEN
        RAISE EXCEPTION 'Authority, configuration and table allocation writes require Scope::HackathonExclusive' USING ERRCODE='42501';
    END IF;
    IF current_data ? 'is_active' AND TG_OP='UPDATE' THEN
        IF (previous->>'is_active')::boolean AND NOT NEW.is_active THEN operation := 'deactivated'; END IF;
        IF NOT (previous->>'is_active')::boolean AND NEW.is_active THEN operation := 'restored'; END IF;
    END IF;
    IF operation='restored' AND explanation IS NULL THEN
        RAISE EXCEPTION 'Restoration requires a reason' USING ERRCODE='23514';
    END IF;
    IF actor IS NULL THEN operation := 'automatic'; END IF;
    IF TG_OP='UPDATE' AND TG_TABLE_NAME IN ('application','project') THEN
        SELECT after_data INTO previous FROM audit_event
            WHERE resource_type=TG_TABLE_NAME::audit_resource AND audit_event.resource_key=resource_identity AND version=OLD.version;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'Previous submission snapshot is missing' USING ERRCODE='23514';
        END IF;
    END IF;
    IF TG_TABLE_NAME='invitation' THEN
        current_data := current_data - 'token_hash';
        previous := previous - 'token_hash';
    ELSIF TG_TABLE_NAME='application' THEN
        current_data := current_data || jsonb_build_object('form_schema',(SELECT application_schema FROM hackathon WHERE id=scope));
    ELSIF TG_TABLE_NAME='project' THEN
        current_data := current_data || jsonb_build_object(
            'form_schema',(SELECT project_schema FROM hackathon WHERE id=scope),
            'track_ids',(SELECT coalesce(jsonb_agg(track_id ORDER BY track_id),'[]') FROM project_track_sub WHERE project_id=NEW.id AND is_active));
    END IF;
    INSERT INTO audit_event(id,hackathon_id,actor_user_id,action,resource_type,resource_key,reason,version,before_data,after_data)
        VALUES(event_id,scope,actor,operation,TG_TABLE_NAME::audit_resource,resource_identity,explanation,NEW.version,previous,current_data);
    RETURN NEW;
END "#,
    },
    Function {
        name: "validate_audit_snapshot",
        security_definer: false,
        body: r#"
DECLARE snapshot audit_event; reference_id uuid;
BEGIN
    IF TG_TABLE_NAME='application' THEN
        FOREACH reference_id IN ARRAY ARRAY[NEW.submitted_audit_id,NEW.reviewed_audit_id] LOOP
            IF reference_id IS NULL THEN CONTINUE; END IF;
            SELECT * INTO snapshot FROM audit_event WHERE id=reference_id;
            IF NOT FOUND OR snapshot.resource_type <> 'application'
                OR snapshot.resource_key <> jsonb_build_object('id',NEW.id)
                OR snapshot.hackathon_id IS DISTINCT FROM NEW.hackathon_id
                OR snapshot.after_data->>'id' IS DISTINCT FROM NEW.id::text
                OR snapshot.after_data->>'hackathon_id' IS DISTINCT FROM NEW.hackathon_id::text
                OR snapshot.after_data->>'submitted_at' IS NULL
                OR snapshot.after_data->>'status' IS NULL THEN
                RAISE EXCEPTION 'Application audit reference must identify its submitted snapshot' USING ERRCODE='23514';
            END IF;
        END LOOP;
    ELSIF NEW.project_audit_id IS NOT NULL THEN
        SELECT * INTO snapshot FROM audit_event WHERE id=NEW.project_audit_id;
        IF NOT FOUND OR snapshot.resource_type <> 'project'
            OR snapshot.resource_key <> jsonb_build_object('id',NEW.project_id)
            OR snapshot.hackathon_id IS DISTINCT FROM NEW.hackathon_id
            OR snapshot.after_data->>'id' IS DISTINCT FROM NEW.project_id::text
            OR snapshot.after_data->>'hackathon_id' IS DISTINCT FROM NEW.hackathon_id::text
            OR snapshot.after_data->>'status' IS DISTINCT FROM 'submitted'
            OR NOT (snapshot.after_data->'track_ids' @> jsonb_build_array(NEW.track_id)) THEN
            RAISE EXCEPTION 'Evaluation audit reference must identify its submitted project' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NEW;
END "#,
    },
    Function {
        name: "validate_team_roster",
        security_definer: false,
        body: r#"
DECLARE team_id_value uuid; roster integer; leaders integer; maximum integer; active_team boolean;
BEGIN
    team_id_value := CASE WHEN TG_TABLE_NAME='team' THEN (to_jsonb(NEW)->>'id')::uuid ELSE (to_jsonb(NEW)->>'team_id')::uuid END;
    SELECT t.is_active,h.max_team_size INTO active_team,maximum FROM team t JOIN hackathon h ON h.id=t.hackathon_id WHERE t.id=team_id_value;
    SELECT count(*),count(*) FILTER (WHERE is_leader) INTO roster,leaders FROM team_member WHERE team_id=team_id_value AND status='active';
    IF roster > maximum OR (active_team AND (roster=0 OR leaders <> 1)) OR (NOT active_team AND roster > 0) THEN
        RAISE EXCEPTION 'Team must respect capacity and have one active leader when nonempty' USING ERRCODE='23514';
    END IF;
    IF EXISTS(SELECT 1 FROM team t JOIN hackathon h ON h.id=t.hackathon_id
        WHERE t.hackathon_id=(SELECT hackathon_id FROM team WHERE id=team_id_value)
            AND t.is_active AND t.table_number IS NOT NULL
        GROUP BY t.table_number,h.max_teams_per_table HAVING count(*) > h.max_teams_per_table) THEN
        RAISE EXCEPTION 'Table allocation exceeds configured capacity' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END "#,
    },
    Function {
        name: "validate_scoped_json",
        security_definer: false,
        body: r#"
DECLARE reference_id uuid; winner jsonb; parent hackathon;
BEGIN
    IF TG_TABLE_NAME='invitation' THEN
        IF EXISTS(SELECT 1 FROM unnest(NEW.track_ids) v(id) LEFT JOIN prize_track p ON p.id=v.id
            WHERE p.id IS NULL OR p.hackathon_id <> NEW.hackathon_id
                OR (NEW.sponsor_org_id IS NOT NULL AND p.sponsor_org_id IS DISTINCT FROM NEW.sponsor_org_id)) THEN
            RAISE EXCEPTION 'Invitation tracks must belong to its hackathon and sponsor scope' USING ERRCODE='23514';
        END IF;
    ELSIF TG_TABLE_NAME='reimbursement_application' THEN
        SELECT * INTO parent FROM hackathon WHERE id=NEW.hackathon_id;
        IF NOT parent.reimbursement_categories ? NEW.category_key THEN
            RAISE EXCEPTION 'Reimbursement category must exist in its hackathon' USING ERRCODE='23514';
        END IF;
    ELSIF TG_TABLE_NAME='hackathon' THEN
        IF jsonb_typeof(NEW.sponsors) <> 'object' OR jsonb_typeof(NEW.reimbursement_categories) <> 'object' THEN
            RAISE EXCEPTION 'Sponsors and reimbursement categories must be objects keyed by stable IDs' USING ERRCODE='23514';
        END IF;
        FOR reference_id IN SELECT jsonb_object_keys(NEW.sponsors)::uuid LOOP
            IF NOT EXISTS(SELECT 1 FROM sponsor_org WHERE id=reference_id) THEN
                RAISE EXCEPTION 'Sponsor association references an unknown organization' USING ERRCODE='23514';
            END IF;
        END LOOP;
        IF EXISTS(SELECT 1 FROM reimbursement_application r WHERE r.hackathon_id=NEW.id AND NOT NEW.reimbursement_categories ? r.category_key) THEN
            RAISE EXCEPTION 'Referenced reimbursement categories cannot be removed' USING ERRCODE='23514';
        END IF;
        IF EXISTS(SELECT 1 FROM team t JOIN team_member m ON m.team_id=t.id AND m.status='active' WHERE t.hackathon_id=NEW.id GROUP BY t.id HAVING count(*) > NEW.max_team_size) THEN
            RAISE EXCEPTION 'Resolve oversized teams before reducing team capacity' USING ERRCODE='23514';
        END IF;
        IF EXISTS(SELECT 1 FROM team WHERE hackathon_id=NEW.id AND is_active AND table_number IS NOT NULL GROUP BY table_number HAVING count(*) > NEW.max_teams_per_table) THEN
            RAISE EXCEPTION 'Resolve table allocations before reducing table capacity' USING ERRCODE='23514';
        END IF;
    ELSIF TG_TABLE_NAME='prize_track' THEN
        IF jsonb_typeof(NEW.winners) <> 'array' OR (NEW.published_winners IS NOT NULL AND jsonb_typeof(NEW.published_winners) <> 'array') THEN
            RAISE EXCEPTION 'Winner lists must be arrays' USING ERRCODE='23514';
        END IF;
        FOR winner IN SELECT * FROM jsonb_array_elements(NEW.winners || coalesce(NEW.published_winners,'[]')) LOOP
            reference_id := (winner->>'project_id')::uuid;
            IF reference_id IS NULL OR NOT EXISTS(SELECT 1 FROM project_track_sub WHERE project_id=reference_id AND track_id=NEW.id AND hackathon_id=NEW.hackathon_id) THEN
                RAISE EXCEPTION 'Winners must reference projects entered in the track' USING ERRCODE='23514';
            END IF;
        END LOOP;
    END IF;
    RETURN NEW;
END "#,
    },
    Function {
        name: "snapshot_project_tracks",
        security_definer: false,
        body: r#"
BEGIN
    UPDATE project SET version=version+1 WHERE id=NEW.project_id;
    RETURN NEW;
END "#,
    },
];
