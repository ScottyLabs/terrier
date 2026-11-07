DO $$
DECLARE t text; populated boolean;
BEGIN
    FOREACH t IN ARRAY ARRAY['hackathon','invitation','user_hackathon_role','application',
        'reimbursement_application','team','team_member','project','prize_track',
        'project_track_sub','judge_track_assign','event','event_check_in','evaluation','sponsor','organizer']
    LOOP
        EXECUTE format('SELECT EXISTS (SELECT 1 FROM %I)', t) INTO populated;
        IF populated THEN
            RAISE EXCEPTION 'RFC 0008 migration needs an explicit legacy mapping for table %. No rows were changed. Supply deadlines, actors, membership times, submission snapshots and scoped grants before retrying.', t;
        END IF;
    END LOOP;
END $$;

CREATE TYPE capability AS ENUM ('application_management','judging_management','sponsorship_management','hackathon_admin');
CREATE TYPE attendance_status AS ENUM ('pending','confirmed','declined','expired','revoked');
CREATE TYPE judge_availability AS ENUM ('available','on_break');
CREATE TYPE project_status AS ENUM ('draft','submitted');
CREATE TYPE audit_action AS ENUM ('created','updated','deactivated','restored','automatic','accessed');
CREATE TYPE audit_resource AS ENUM ('user','hackathon','invitation','user_hackathon_role',
    'application','reimbursement_application','team','team_member','project','sponsor_org',
    'prize_track','project_track_sub','judge_track_assign','event','event_check_in','evaluation','sponsor','organizer');
ALTER TYPE team_member_status RENAME TO team_member_status_legacy;
CREATE TYPE team_member_status AS ENUM ('active','invited','removed','pending','withdrawn','rejected');
ALTER TABLE team_member ALTER COLUMN status TYPE team_member_status USING status::text::team_member_status;
DROP TYPE team_member_status_legacy;
ALTER TYPE reimbursement_status RENAME TO reimbursement_status_legacy;
CREATE TYPE reimbursement_status AS ENUM ('pending','approved','rejected','paid','denied','withdrawn');
ALTER TABLE reimbursement_application ALTER COLUMN status TYPE reimbursement_status USING status::text::reimbursement_status;
DROP TYPE reimbursement_status_legacy;
ALTER TABLE "user" ADD COLUMN profile_data jsonb NOT NULL DEFAULT '{}';
ALTER TABLE "user" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "hackathon" ADD COLUMN completed_at timestamptz;
ALTER TABLE "hackathon" ADD COLUMN max_teams_per_table integer NOT NULL DEFAULT 1 CHECK (max_teams_per_table > 0);
ALTER TABLE "hackathon" ADD COLUMN application_deadline timestamptz NOT NULL;
ALTER TABLE "hackathon" ADD COLUMN team_formation_deadline timestamptz NOT NULL;
ALTER TABLE "hackathon" ADD COLUMN submission_deadline timestamptz NOT NULL;
ALTER TABLE "hackathon" ADD COLUMN project_schema jsonb NOT NULL DEFAULT '{}';
ALTER TABLE "hackathon" ADD COLUMN directory_schema jsonb NOT NULL DEFAULT '{}';
ALTER TABLE "hackathon" ADD COLUMN directory_fields jsonb NOT NULL DEFAULT '[]';
ALTER TABLE "hackathon" ADD COLUMN reimbursement_categories jsonb NOT NULL DEFAULT '{}';
ALTER TABLE "hackathon" ADD COLUMN sponsors jsonb NOT NULL DEFAULT '{}';
ALTER TABLE "hackathon" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "invitation" ADD COLUMN issuer_user_id uuid NOT NULL REFERENCES "user"(id);
ALTER TABLE "invitation" ADD COLUMN target_user_id uuid NOT NULL REFERENCES "user"(id);
ALTER TABLE "invitation" ADD COLUMN token_hash text NOT NULL UNIQUE;
ALTER TABLE "invitation" ADD COLUMN capabilities capability[] NOT NULL DEFAULT '{}';
ALTER TABLE "invitation" ADD COLUMN track_ids uuid[] NOT NULL DEFAULT '{}';
ALTER TABLE "invitation" ADD COLUMN sponsor_org_id uuid REFERENCES sponsor_org(id);
ALTER TABLE "invitation" ADD COLUMN expires_at timestamptz NOT NULL;
ALTER TABLE "invitation" ADD COLUMN revoked_at timestamptz;
ALTER TABLE "invitation" ADD COLUMN redeemed_at timestamptz;
ALTER TABLE "invitation" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "user_hackathon_role" ADD COLUMN profile_data jsonb NOT NULL DEFAULT '{}';
ALTER TABLE "user_hackathon_role" ADD COLUMN directory_opt_in boolean NOT NULL DEFAULT false;
ALTER TABLE "user_hackathon_role" ADD COLUMN is_active boolean NOT NULL DEFAULT true;
ALTER TABLE "user_hackathon_role" ADD COLUMN judge_availability judge_availability NOT NULL DEFAULT 'available';
ALTER TABLE "user_hackathon_role" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "application" ADD COLUMN draft_data jsonb NOT NULL DEFAULT '{}';
ALTER TABLE "application" ADD COLUMN submitted_audit_id uuid;
ALTER TABLE "application" ADD COLUMN reviewed_audit_id uuid;
ALTER TABLE "application" ADD COLUMN decided_by uuid REFERENCES "user"(id);
ALTER TABLE "application" ADD COLUMN submitted_at timestamptz;
ALTER TABLE "application" ADD COLUMN decided_at timestamptz;
ALTER TABLE "application" ADD COLUMN decision_reason text;
ALTER TABLE "application" ADD COLUMN attendance_status attendance_status;
ALTER TABLE "application" ADD COLUMN confirmation_deadline timestamptz;
ALTER TABLE "application" ADD COLUMN is_active boolean NOT NULL DEFAULT true;
ALTER TABLE "application" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "reimbursement_application" ADD COLUMN category_key text NOT NULL;
ALTER TABLE "reimbursement_application" ADD COLUMN hackathon_id uuid NOT NULL REFERENCES hackathon(id);
ALTER TABLE "reimbursement_application" ADD COLUMN evidence jsonb NOT NULL DEFAULT '{}';
ALTER TABLE "reimbursement_application" ADD COLUMN approved_amount numeric;
ALTER TABLE "reimbursement_application" ADD COLUMN currency text;
ALTER TABLE "reimbursement_application" ADD COLUMN decided_by uuid REFERENCES "user"(id);
ALTER TABLE "reimbursement_application" ADD COLUMN decided_at timestamptz;
ALTER TABLE "reimbursement_application" ADD COLUMN decision_reason text;
ALTER TABLE "reimbursement_application" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "team" ADD COLUMN description text NOT NULL DEFAULT '';
ALTER TABLE "team" ADD COLUMN is_active boolean NOT NULL DEFAULT true;
ALTER TABLE "team" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "team_member" ADD COLUMN hackathon_id uuid NOT NULL REFERENCES hackathon(id);
ALTER TABLE "team_member" ADD COLUMN is_leader boolean NOT NULL DEFAULT false;
ALTER TABLE "team_member" ADD COLUMN requested_at timestamptz NOT NULL;
ALTER TABLE "team_member" ADD COLUMN joined_at timestamptz;
ALTER TABLE "team_member" ADD COLUMN ended_at timestamptz;
ALTER TABLE "team_member" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "project" ADD COLUMN status project_status NOT NULL DEFAULT 'draft';
ALTER TABLE "project" ADD COLUMN submitted_at timestamptz;
ALTER TABLE "project" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "prize_track" ADD COLUMN editing_locked boolean NOT NULL DEFAULT false;
ALTER TABLE "prize_track" ADD COLUMN criteria jsonb NOT NULL DEFAULT '{}';
ALTER TABLE "prize_track" ADD COLUMN eligibility jsonb NOT NULL DEFAULT '{}';
ALTER TABLE "prize_track" ADD COLUMN winners jsonb NOT NULL DEFAULT '[]';
ALTER TABLE "prize_track" ADD COLUMN published_winners jsonb;
ALTER TABLE "prize_track" ADD COLUMN release_at timestamptz;
ALTER TABLE "prize_track" ADD COLUMN published_by uuid REFERENCES "user"(id);
ALTER TABLE "prize_track" ADD COLUMN publication_revoked_at timestamptz;
ALTER TABLE "prize_track" ADD COLUMN is_active boolean NOT NULL DEFAULT true;
ALTER TABLE "prize_track" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "project_track_sub" ADD COLUMN hackathon_id uuid NOT NULL REFERENCES hackathon(id);
ALTER TABLE "project_track_sub" ADD COLUMN is_active boolean NOT NULL DEFAULT true;
ALTER TABLE "project_track_sub" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "judge_track_assign" ADD COLUMN hackathon_id uuid NOT NULL REFERENCES hackathon(id);
ALTER TABLE "judge_track_assign" ADD COLUMN manually_granted boolean NOT NULL DEFAULT false;
ALTER TABLE "judge_track_assign" ADD COLUMN sponsor_granted boolean NOT NULL DEFAULT false;
ALTER TABLE "judge_track_assign" ADD COLUMN is_selected boolean NOT NULL DEFAULT false;
ALTER TABLE "judge_track_assign" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "event" ADD COLUMN is_active boolean NOT NULL DEFAULT true;
ALTER TABLE "event" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "event_check_in" ADD COLUMN is_valid boolean NOT NULL DEFAULT true;
ALTER TABLE "event_check_in" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "evaluation" ADD COLUMN hackathon_id uuid NOT NULL REFERENCES hackathon(id);
ALTER TABLE "evaluation" ADD COLUMN project_audit_id uuid;
ALTER TABLE "evaluation" ADD COLUMN rubric jsonb NOT NULL DEFAULT '{}';
ALTER TABLE "evaluation" ADD COLUMN manual_override boolean NOT NULL DEFAULT false;
ALTER TABLE "evaluation" ADD COLUMN ended_at timestamptz;
ALTER TABLE "evaluation" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "sponsor" ADD COLUMN hackathon_id uuid NOT NULL REFERENCES hackathon(id);
ALTER TABLE "sponsor" ADD COLUMN is_active boolean NOT NULL DEFAULT true;
ALTER TABLE "sponsor" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "organizer" ADD COLUMN capabilities capability[] NOT NULL DEFAULT '{}';
ALTER TABLE "organizer" ADD COLUMN is_active boolean NOT NULL DEFAULT true;
ALTER TABLE "organizer" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);
ALTER TABLE "sponsor_org" ADD COLUMN version integer NOT NULL DEFAULT 1 CHECK (version > 0);

ALTER TABLE hackathon DROP COLUMN travel_reimbursement_enabled;
ALTER TABLE invitation RENAME COLUMN role TO initial_role;
ALTER TABLE invitation ALTER COLUMN initial_role DROP NOT NULL;
ALTER TABLE invitation DROP COLUMN code, DROP COLUMN target_email, DROP COLUMN is_used, DROP COLUMN created_at;
ALTER TABLE application ALTER COLUMN status DROP NOT NULL;
ALTER TABLE prize_track ALTER COLUMN sponsor_org_id DROP NOT NULL;
ALTER TABLE evaluation ALTER COLUMN evaluation DROP NOT NULL;
ALTER TABLE sponsor DROP CONSTRAINT sponsor_pkey;
ALTER TABLE sponsor ADD PRIMARY KEY (user_id, hackathon_id, sponsor_org_id);
ALTER TABLE hackathon ADD UNIQUE (slug), ADD CHECK (max_team_size > 0 AND max_participants > 0);
ALTER TABLE application ADD UNIQUE (user_id,hackathon_id), ADD UNIQUE(id,hackathon_id);
ALTER TABLE reimbursement_application ADD UNIQUE(application_id,category_key);
ALTER TABLE team ADD UNIQUE(id,hackathon_id);
ALTER TABLE event ADD UNIQUE(id,hackathon_id);
ALTER TABLE project ADD UNIQUE(team_id,hackathon_id), ADD UNIQUE(id,hackathon_id);
ALTER TABLE prize_track ADD UNIQUE(id,hackathon_id);
ALTER TABLE project_track_sub ADD UNIQUE(project_id,track_id,hackathon_id);
ALTER TABLE judge_track_assign ADD UNIQUE(user_id,track_id,hackathon_id);
ALTER TABLE event_check_in ADD UNIQUE(event_id,user_id);
CREATE UNIQUE INDEX team_member_current_user ON team_member(user_id,hackathon_id) WHERE status IN ('active','pending');
CREATE UNIQUE INDEX team_member_one_leader ON team_member(team_id) WHERE status='active' AND is_leader;
ALTER TABLE team_member ADD CHECK (NOT is_leader OR status='active');
ALTER TABLE team_member ADD CHECK ((status='active' AND joined_at IS NOT NULL AND ended_at IS NULL)
    OR (status='pending' AND joined_at IS NULL AND ended_at IS NULL)
    OR (status IN ('removed','withdrawn','rejected') AND ended_at IS NOT NULL AND NOT is_leader)
    OR status='invited');
ALTER TABLE project ADD CHECK ((status='draft' AND submitted_at IS NULL) OR (status='submitted' AND submitted_at IS NOT NULL));
ALTER TABLE application ADD CHECK ((status IS NULL AND submitted_at IS NULL AND submitted_audit_id IS NULL)
    OR (status IS NOT NULL AND submitted_at IS NOT NULL AND submitted_audit_id IS NOT NULL));
ALTER TABLE application ADD CHECK (attendance_status IS NULL OR status='accepted');
ALTER TABLE reimbursement_application ADD CHECK (approved_amount IS NULL OR approved_amount >= 0);
ALTER TABLE reimbursement_application ADD CHECK ((approved_amount IS NULL) = (currency IS NULL));
ALTER TABLE reimbursement_application ADD CHECK (status <> 'approved' OR (approved_amount IS NOT NULL AND currency IS NOT NULL));
ALTER TABLE reimbursement_application ADD CHECK (currency IS NULL OR currency ~ '^[A-Z]{3}$');
ALTER TABLE invitation ADD CHECK (expires_at > '-infinity'::timestamptz AND expires_at < 'infinity'::timestamptz);
ALTER TABLE invitation ADD CHECK (array_position(capabilities,NULL) IS NULL AND array_position(track_ids,NULL) IS NULL);
ALTER TABLE organizer ADD CHECK (array_position(capabilities,NULL) IS NULL);
ALTER TABLE evaluation ADD CHECK (status <> 'complete' OR (evaluation IS NOT NULL AND project_audit_id IS NOT NULL));
CREATE UNIQUE INDEX evaluation_current_assignment ON evaluation(judge_id,project_id,track_id) WHERE ended_at IS NULL;
ALTER TABLE application ADD CONSTRAINT scope_application_0 FOREIGN KEY (user_id,hackathon_id) REFERENCES user_hackathon_role(user_id,hackathon_id);
ALTER TABLE reimbursement_application ADD CONSTRAINT scope_reimbursement_application_1 FOREIGN KEY (application_id,hackathon_id) REFERENCES application(id,hackathon_id);
ALTER TABLE team_member ADD CONSTRAINT scope_team_member_2 FOREIGN KEY (team_id,hackathon_id) REFERENCES team(id,hackathon_id);
ALTER TABLE team_member ADD CONSTRAINT scope_team_member_3 FOREIGN KEY (user_id,hackathon_id) REFERENCES user_hackathon_role(user_id,hackathon_id);
ALTER TABLE project ADD CONSTRAINT scope_project_4 FOREIGN KEY (team_id,hackathon_id) REFERENCES team(id,hackathon_id);
ALTER TABLE project ADD CONSTRAINT scope_project_5 FOREIGN KEY (event_id,hackathon_id) REFERENCES event(id,hackathon_id);
ALTER TABLE project_track_sub ADD CONSTRAINT scope_project_track_sub_6 FOREIGN KEY (project_id,hackathon_id) REFERENCES project(id,hackathon_id);
ALTER TABLE project_track_sub ADD CONSTRAINT scope_project_track_sub_7 FOREIGN KEY (track_id,hackathon_id) REFERENCES prize_track(id,hackathon_id);
ALTER TABLE judge_track_assign ADD CONSTRAINT scope_judge_track_assign_8 FOREIGN KEY (user_id,hackathon_id) REFERENCES user_hackathon_role(user_id,hackathon_id);
ALTER TABLE judge_track_assign ADD CONSTRAINT scope_judge_track_assign_9 FOREIGN KEY (track_id,hackathon_id) REFERENCES prize_track(id,hackathon_id);
ALTER TABLE evaluation ADD CONSTRAINT scope_evaluation_10 FOREIGN KEY (project_id,track_id,hackathon_id) REFERENCES project_track_sub(project_id,track_id,hackathon_id);
ALTER TABLE evaluation ADD CONSTRAINT scope_evaluation_11 FOREIGN KEY (judge_id,track_id,hackathon_id) REFERENCES judge_track_assign(user_id,track_id,hackathon_id);
ALTER TABLE sponsor ADD CONSTRAINT scope_sponsor_12 FOREIGN KEY (user_id,hackathon_id) REFERENCES user_hackathon_role(user_id,hackathon_id);
ALTER TABLE organizer ADD CONSTRAINT scope_organizer_13 FOREIGN KEY (user_id,hackathon_id) REFERENCES user_hackathon_role(user_id,hackathon_id);

CREATE TABLE audit_event (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    hackathon_id uuid REFERENCES hackathon(id) DEFERRABLE INITIALLY DEFERRED,
    actor_user_id uuid REFERENCES "user"(id) DEFERRABLE INITIALLY DEFERRED,
    action audit_action NOT NULL,
    resource_type audit_resource NOT NULL,
    resource_key jsonb NOT NULL CHECK (jsonb_typeof(resource_key)='object' AND resource_key <> '{}'),
    occurred_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    reason text,
    version integer CHECK (version > 0),
    before_data jsonb,
    after_data jsonb,
    CHECK (actor_user_id IS NOT NULL OR (action='automatic' AND reason IS NOT NULL AND length(btrim(reason)) > 0)),
    CHECK (action='accessed' OR (version IS NOT NULL AND after_data IS NOT NULL)),
    UNIQUE(resource_type,resource_key,version)
);
CREATE INDEX audit_event_scope_time ON audit_event(hackathon_id,occurred_at);
ALTER TABLE application ADD FOREIGN KEY (submitted_audit_id) REFERENCES audit_event(id) DEFERRABLE INITIALLY DEFERRED;
ALTER TABLE application ADD FOREIGN KEY (reviewed_audit_id) REFERENCES audit_event(id) DEFERRABLE INITIALLY DEFERRED;
ALTER TABLE evaluation ADD FOREIGN KEY (project_audit_id) REFERENCES audit_event(id) DEFERRABLE INITIALLY DEFERRED;
