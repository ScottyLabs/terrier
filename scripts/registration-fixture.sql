\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('terrier.automatic', 'true', true),
       set_config('terrier.audit_reason', 'Local registration fixture', true);
INSERT INTO hackathon (
    id, slug, name, description, is_active, max_team_size, max_participants,
    location, contact_email, application_schema, data, application_deadline,
    team_formation_deadline, submission_deadline
) VALUES (
    gen_random_uuid(), 'registration-test', 'Registration test', 'Local fixture',
    true, 4, 100, 'Campus', 'test@example.com',
    '{"fields":[{"id":"school","label":"School","type":"text","required":true},{"id":"resume","label":"Resume","type":"file","required":true}]}',
    '{}', clock_timestamp() + interval '7 days', clock_timestamp() + interval '8 days',
    clock_timestamp() + interval '9 days'
)
ON CONFLICT (slug) DO NOTHING;

SELECT set_config('terrier.hackathon_id', id::text, true),
       set_config('terrier.management', 'true', true)
FROM hackathon WHERE slug = 'registration-test';
INSERT INTO user_hackathon_role (user_id, hackathon_id, role)
VALUES (:'reviewer_id'::uuid, current_setting('terrier.hackathon_id')::uuid, 'organizer')
ON CONFLICT (user_id, hackathon_id) DO NOTHING;
INSERT INTO organizer (user_id, hackathon_id, capabilities)
VALUES (:'reviewer_id'::uuid, current_setting('terrier.hackathon_id')::uuid,
        ARRAY['hackathon_admin', 'application_management']::capability[])
ON CONFLICT (user_id, hackathon_id) DO NOTHING;
COMMIT;
