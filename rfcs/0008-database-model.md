# RFC 0008: Database Model

- **Status:** Accepted
- **Author(s):** @kritdass
- **Created:** 2026-03-28
- **Updated:** 2026-06-16

## Overview

This RFC establishes the database model that the Terrier application will use.

## Motivation

Terrier is a very complex application, requiring several interconnected tables with related data to work. Having a strictly defined, scalable schema and a clear workflow for generating database entities is critical for maintaining data integrity and accelerating backend development.

## Goals

- Define the core entities, relationships, and schema for the Terrier application
- Establish a single source of truth for the database architecture
- Automate the generation of Rust database entities to minimize boilerplate
- Ensure type-safe, asynchronous database interactions using SeaORM

## Non-Goals

- Deciding on the physical database hosting provider or infrastructure
- Defining the exact business logic or API routes that will interact with this data
- Writing raw SQL queries for standard CRUD operations

## Detailed Design

The Terrier database model is defined and maintained as a Mermaid entity-relationship diagram embedded in this document. The development workflow will rely on a code-first migration approach paired with automated entity generation to ensure our Rust backend stays perfectly in sync with the database schema.

### Schema Source of Truth

The entity-relationship diagram below is the canonical reference for all developers when planning feature integrations. Changes to the schema must be reflected here and in a corresponding SeaORM migration.

```mermaid
erDiagram
    User {
        UUID id
        String email
        String display_name
        Boolean is_global_admin
    }

    Hackathon {
        UUID id
        String slug
        String name
        String description
        Boolean is_active
        Int max_team_size
        Int max_participants
        String location
        String contact_email
        Boolean travel_reimbursement_enabled
        JSONB application_schema
        JSONB data
    }

    Invitation {
        UUID id
        UUID hackathon_id
        String code
        Enum role
        NullableString target_email
        Boolean is_used
        DateTime created_at
    }

    UserHackathonRole {
        UUID user_id
        UUID hackathon_id
        Enum role
    }

    Application {
        UUID id
        UUID user_id
        UUID hackathon_id
        Enum status
        JSONB data
    }

    ReimbursementApplication {
        UUID id
        UUID application_id
        Enum status
        JSONB data
    }

    Team {
        UUID id
        UUID hackathon_id
        String name
        NullableString table_number
    }

    TeamMember {
        UUID team_id
        UUID user_id
        Enum status
    }

    Project {
        UUID id
        UUID hackathon_id
        UUID event_id
        UUID team_id
        JSONB data
    }

    SponsorOrg {
        UUID id
        String name
        String description
    }

    PrizeTrack {
        UUID id
        UUID hackathon_id
        UUID sponsor_org_id
        String name
        String description
        Boolean is_judging_started
        Enum judging_method
    }

    ProjectTrackSub {
        UUID project_id
        UUID track_id
    }

    JudgeTrackAssign {
        UUID user_id
        UUID track_id
    }

    Event {
        UUID id
        UUID hackathon_id
        String name
        String description
        DateTime start_time
        DateTime end_time
        String location
    }

    EventCheckIn {
        UUID id
        UUID event_id
        UUID user_id
        DateTime check_in_time
        UUID checked_in_by
    }

    Evaluation {
        UUID id
        UUID track_id
        UUID project_id
        UUID judge_id
        JSONB evaluation
        Enum judging_method
        Enum status
        DateTime created_at
    }

    Sponsor {
        UUID user_id
        UUID sponsor_org_id
    }

    Organizer {
        UUID user_id
        UUID hackathon_id
    }

    %% Relationships
    User ||--o{ UserHackathonRole : "has_many"
    Hackathon ||--o{ UserHackathonRole : "has_many"
    Hackathon ||--o{ Invitation : "has_many"

    User ||--o{ Application : "has_many"
    Hackathon ||--o{ Application : "has_many"
    Application ||--o| ReimbursementApplication : "can_have"

    Hackathon ||--o{ Team : "has_many"
    Team ||--o{ TeamMember : "has_many"
    User ||--o{ TeamMember : "has_many"

    Team ||--o| Project : "has_one"
    Hackathon ||--o{ Project : "has_many"
    Event ||--o{ Project : "has_many"

    Hackathon ||--o{ PrizeTrack : "has_many"
    SponsorOrg ||--o{ PrizeTrack : "has_many"

    SponsorOrg ||--o{ Sponsor : "has_many"
    User ||--o{ Sponsor : "has_many"

    Hackathon ||--o{ Organizer : "has_many"
    User ||--o{ Organizer : "has_many"

    Project ||--o{ ProjectTrackSub : "has_many"
    PrizeTrack ||--o{ ProjectTrackSub : "has_many"

    User ||--o{ JudgeTrackAssign : "has_many"
    PrizeTrack ||--o{ JudgeTrackAssign : "has_many"

    Hackathon ||--o{ Event : "has_many"
    Event ||--o{ EventCheckIn : "has_many"
    User ||--o{ EventCheckIn : "has_many"
    User ||--o{ EventCheckIn : "checked_in_by"

    Project ||--o{ Evaluation : "has_many"
    PrizeTrack ||--o{ Evaluation : "has_many"
    User ||--o{ Evaluation : "has_many"
```

### Enum Reference

All PostgreSQL enum types used in the schema are defined here.

| Type | Values |
|---|---|
| `hackathon_role` | `hacker`, `organizer`, `judge`, `sponsor` |
| `application_status` | `pending`, `accepted`, `rejected`, `waitlisted` |
| `reimbursement_status` | `pending`, `approved`, `rejected`, `paid` |
| `team_member_status` | `active`, `invited`, `removed` |
| `judging_method` | `scoring`, `expo` |
| `evaluation_status` | `pending`, `complete`, `skipped` |

`hackathon_role` is shared between `UserHackathonRole.role` and `Invitation.role`. `judging_method` is shared between `PrizeTrack.judging_method` and `Evaluation.judging_method`.

### ORM and Tooling Workflow

We are utilizing **Rust** as our backend language alongside **SeaORM**, an async, dynamic ORM. The workflow for interacting with and updating the database model is as follows:

1. **Migrations:** Schema changes (creating tables, altering columns, defining foreign keys) will be written using SeaORM's migration system. This ensures a version-controlled, reproducible database state across all environments.
1. **Entity Generation:** Instead of manually writing Rust structs for our database tables, we will use the `sea-orm-cli`. After running migrations, developers will run the CLI tool to automatically introspect the database and generate the corresponding Rust entities, schemas, and relational bindings.
1. **Type Safety:** By relying on the CLI, we guarantee that our Rust backend types strictly match our database columns, preventing runtime serialization errors.
