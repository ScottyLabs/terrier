-- Terrier Platform SQL Schema
-- Generated from SeaORM entity definitions
-- Database: PostgreSQL

-- ============================================
-- Core Tables
-- ============================================

-- Users table: stores all user accounts (authenticated via OIDC)
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    oidc_sub VARCHAR(255) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL,
    name VARCHAR(255),
    given_name VARCHAR(255),
    family_name VARCHAR(255),
    picture TEXT,
    oidc_issuer VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Hackathons table: represents individual hackathon events
CREATE TABLE hackathons (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    start_date TIMESTAMP NOT NULL,
    end_date TIMESTAMP NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    max_team_size INTEGER NOT NULL DEFAULT 4,
    banner_url VARCHAR(512),
    form_config JSON,
    background_url VARCHAR(512),
    submission_form JSONB,
    app_icon_url VARCHAR(512),
    theme_color VARCHAR(32),
    background_color VARCHAR(32),
    submissions_closed BOOLEAN NOT NULL DEFAULT FALSE,
    judging_started BOOLEAN NOT NULL DEFAULT FALSE,
    judge_session_timeout_minutes INTEGER NOT NULL DEFAULT 30
);

-- ============================================
-- Teams & Membership
-- ============================================

-- Teams table: hackathon participant teams
CREATE TABLE teams (
    id SERIAL PRIMARY KEY,
    hackathon_id INTEGER NOT NULL REFERENCES hackathons(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    owner_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX idx_teams_hackathon_id ON teams(hackathon_id);
CREATE INDEX idx_teams_owner_id ON teams(owner_id);

-- Team Members table: unified table for team membership and requests
CREATE TABLE team_members (
    id SERIAL PRIMARY KEY,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL,           -- 'owner', 'member', etc.
    status VARCHAR(50) NOT NULL,         -- 'pending', 'accepted', 'rejected'
    request_type VARCHAR(50) NOT NULL,   -- 'invitation', 'join_request'
    initiated_by_user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    responded_at TIMESTAMP
);

CREATE INDEX idx_team_members_team_id ON team_members(team_id);
CREATE INDEX idx_team_members_user_id ON team_members(user_id);

-- Team Invitations table: pending invitations to join a team
CREATE TABLE team_invitations (
    id SERIAL PRIMARY KEY,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_team_invitations_team_id ON team_invitations(team_id);
CREATE INDEX idx_team_invitations_user_id ON team_invitations(user_id);

-- Team Join Requests table: user requests to join a team
CREATE TABLE team_join_requests (
    id SERIAL PRIMARY KEY,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_team_join_requests_team_id ON team_join_requests(team_id);
CREATE INDEX idx_team_join_requests_user_id ON team_join_requests(user_id);

-- ============================================
-- Applications & Roles
-- ============================================

-- Applications table: user applications to hackathons
CREATE TABLE applications (
    id SERIAL PRIMARY KEY,
    hackathon_id INTEGER NOT NULL REFERENCES hackathons(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    form_data JSON NOT NULL,
    status VARCHAR(50) NOT NULL,  -- 'draft', 'submitted', 'accepted', 'rejected', 'waitlisted', 'confirmed'
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (hackathon_id, user_id)
);

CREATE INDEX idx_applications_hackathon_id ON applications(hackathon_id);
CREATE INDEX idx_applications_user_id ON applications(user_id);
CREATE INDEX idx_applications_status ON applications(status);

-- User Hackathon Roles table: user roles within a hackathon
CREATE TABLE user_hackathon_roles (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    hackathon_id INTEGER NOT NULL REFERENCES hackathons(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL,  -- 'participant', 'organizer', 'admin', 'judge', etc.
    team_id INTEGER REFERENCES teams(id) ON DELETE SET NULL
);

CREATE INDEX idx_user_hackathon_roles_user_id ON user_hackathon_roles(user_id);
CREATE INDEX idx_user_hackathon_roles_hackathon_id ON user_hackathon_roles(hackathon_id);
CREATE UNIQUE INDEX idx_user_hackathon_roles_unique ON user_hackathon_roles(user_id, hackathon_id, role);

-- ============================================
-- Events & Check-ins
-- ============================================

-- Events table: hackathon schedule events (workshops, meals, etc.)
CREATE TABLE events (
    id SERIAL PRIMARY KEY,
    hackathon_id INTEGER NOT NULL REFERENCES hackathons(id),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL,
    description TEXT,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP NOT NULL,
    visible_to_role VARCHAR(50),  -- null = visible to all
    event_type VARCHAR(50) NOT NULL,  -- 'workshop', 'meal', 'ceremony', etc.
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    location VARCHAR(255),
    is_visible BOOLEAN NOT NULL DEFAULT TRUE,
    points INTEGER,
    checkin_type VARCHAR(50) NOT NULL  -- 'none', 'qr', 'manual'
);

CREATE INDEX idx_events_hackathon_id ON events(hackathon_id);
CREATE INDEX idx_events_start_time ON events(start_time);
CREATE UNIQUE INDEX idx_events_hackathon_slug ON events(hackathon_id, slug);

-- Event Organizers table: users who can manage specific events
CREATE TABLE event_organizers (
    id SERIAL PRIMARY KEY,
    event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_event_organizers_event_id ON event_organizers(event_id);
CREATE INDEX idx_event_organizers_user_id ON event_organizers(user_id);

-- Event Check-ins table: tracks user attendance at events
CREATE TABLE event_checkins (
    id SERIAL PRIMARY KEY,
    event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL,
    checked_in_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    checked_in_by INTEGER  -- organizer who checked them in (null if self-checkin)
);

CREATE INDEX idx_event_checkins_event_id ON event_checkins(event_id);
CREATE INDEX idx_event_checkins_user_id ON event_checkins(user_id);
CREATE UNIQUE INDEX idx_event_checkins_unique ON event_checkins(event_id, user_id);

-- ============================================
-- Submissions
-- ============================================

-- Submission table: project submissions from teams
CREATE TABLE submission (
    id SERIAL PRIMARY KEY,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    submission_data JSONB NOT NULL,
    submitted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_submission_team_id ON submission(team_id);
CREATE UNIQUE INDEX idx_submission_team_unique ON submission(team_id);

-- Submission AI Summary table: AI-generated project summaries
CREATE TABLE submission_ai_summary (
    id SERIAL PRIMARY KEY,
    submission_id INTEGER NOT NULL UNIQUE REFERENCES submission(id) ON DELETE CASCADE,
    summary TEXT,
    generated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ============================================
-- Judging System
-- ============================================

-- Feature table: judging criteria/dimensions
CREATE TABLE feature (
    id SERIAL PRIMARY KEY,
    hackathon_id INTEGER NOT NULL REFERENCES hackathons(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT
);

CREATE INDEX idx_feature_hackathon_id ON feature(hackathon_id);

-- Judge Feature Assignment table: assigns judges to specific features
CREATE TABLE judge_feature_assignment (
    id SERIAL PRIMARY KEY,
    judge_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    feature_id INTEGER NOT NULL REFERENCES feature(id) ON DELETE CASCADE,
    current_best_submission_id INTEGER REFERENCES submission(id) ON DELETE SET NULL,
    notes TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_judge_feature_assignment_judge_id ON judge_feature_assignment(judge_id);
CREATE INDEX idx_judge_feature_assignment_feature_id ON judge_feature_assignment(feature_id);
CREATE UNIQUE INDEX idx_judge_feature_assignment_unique ON judge_feature_assignment(judge_id, feature_id);

-- Project Visit table: tracks judge visits to projects
CREATE TABLE project_visit (
    id SERIAL PRIMARY KEY,
    submission_id INTEGER NOT NULL REFERENCES submission(id) ON DELETE CASCADE,
    judge_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    hackathon_id INTEGER NOT NULL REFERENCES hackathons(id) ON DELETE CASCADE,
    notes TEXT,
    start_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completion_time TIMESTAMP,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX idx_project_visit_submission_id ON project_visit(submission_id);
CREATE INDEX idx_project_visit_judge_id ON project_visit(judge_id);
CREATE INDEX idx_project_visit_hackathon_id ON project_visit(hackathon_id);
CREATE INDEX idx_project_visit_active ON project_visit(is_active) WHERE is_active = TRUE;

-- Pairwise Comparison table: judge comparisons between two projects
CREATE TABLE pairwise_comparison (
    id SERIAL PRIMARY KEY,
    judge_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    feature_id INTEGER NOT NULL REFERENCES feature(id) ON DELETE CASCADE,
    submission_a_id INTEGER NOT NULL REFERENCES submission(id) ON DELETE CASCADE,
    submission_b_id INTEGER NOT NULL REFERENCES submission(id) ON DELETE CASCADE,
    winner_id INTEGER,  -- null = tie or skipped
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_pairwise_comparison_judge_id ON pairwise_comparison(judge_id);
CREATE INDEX idx_pairwise_comparison_feature_id ON pairwise_comparison(feature_id);
CREATE INDEX idx_pairwise_comparison_submissions ON pairwise_comparison(submission_a_id, submission_b_id);

-- Project Feature Score table: calculated scores per feature
CREATE TABLE project_feature_score (
    id SERIAL PRIMARY KEY,
    submission_id INTEGER NOT NULL REFERENCES submission(id) ON DELETE CASCADE,
    feature_id INTEGER NOT NULL REFERENCES feature(id) ON DELETE CASCADE,
    score FLOAT,
    variance FLOAT,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_project_feature_score_submission_id ON project_feature_score(submission_id);
CREATE INDEX idx_project_feature_score_feature_id ON project_feature_score(feature_id);
CREATE UNIQUE INDEX idx_project_feature_score_unique ON project_feature_score(submission_id, feature_id);

-- ============================================
-- Prizes
-- ============================================

-- Prize table: hackathon prizes
CREATE TABLE prize (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    image_url VARCHAR(512),
    category VARCHAR(100),
    value VARCHAR(100) NOT NULL,
    hackathon_id INTEGER REFERENCES hackathons(id) ON DELETE CASCADE
);

CREATE INDEX idx_prize_hackathon_id ON prize(hackathon_id);

-- Prize Feature Weight table: how features contribute to prize ranking
CREATE TABLE prize_feature_weight (
    id SERIAL PRIMARY KEY,
    prize_id INTEGER NOT NULL REFERENCES prize(id) ON DELETE CASCADE,
    feature_id INTEGER NOT NULL REFERENCES feature(id) ON DELETE CASCADE,
    weight FLOAT NOT NULL DEFAULT 1.0
);

CREATE INDEX idx_prize_feature_weight_prize_id ON prize_feature_weight(prize_id);
CREATE INDEX idx_prize_feature_weight_feature_id ON prize_feature_weight(feature_id);
CREATE UNIQUE INDEX idx_prize_feature_weight_unique ON prize_feature_weight(prize_id, feature_id);

-- Prize Track Entry table: submissions competing for specific prizes
CREATE TABLE prize_track_entry (
    id SERIAL PRIMARY KEY,
    submission_id INTEGER NOT NULL REFERENCES submission(id) ON DELETE CASCADE,
    prize_id INTEGER NOT NULL REFERENCES prize(id) ON DELETE CASCADE
);

CREATE INDEX idx_prize_track_entry_submission_id ON prize_track_entry(submission_id);
CREATE INDEX idx_prize_track_entry_prize_id ON prize_track_entry(prize_id);
CREATE UNIQUE INDEX idx_prize_track_entry_unique ON prize_track_entry(submission_id, prize_id);

-- ============================================
-- Utility Indexes for Common Queries
-- ============================================

-- Index for finding active hackathons
CREATE INDEX idx_hackathons_active ON hackathons(is_active) WHERE is_active = TRUE;

-- Index for finding pending applications
CREATE INDEX idx_applications_pending ON applications(hackathon_id, status) WHERE status = 'submitted';
