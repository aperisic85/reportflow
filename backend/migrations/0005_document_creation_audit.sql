-- Explicit creation times and actors, independently of later edits or submissions.
-- Historical rows did not record creation dates: for legacy update rows we use
-- the first surviving revision timestamp (or last update); these are estimates.
ALTER TABLE activity_updates ADD COLUMN created_at TIMESTAMPTZ;
ALTER TABLE activity_updates ADD COLUMN created_by UUID REFERENCES users(id);
ALTER TABLE activity_updates ADD COLUMN creation_time_estimated BOOLEAN NOT NULL DEFAULT FALSE;
UPDATE activity_updates u SET
 created_at=COALESCE((SELECT MIN(r.changed_at) FROM activity_update_revisions r WHERE r.update_id=u.id),u.updated_at),
 created_by=u.updated_by,
 creation_time_estimated=TRUE;
ALTER TABLE activity_updates ALTER COLUMN created_at SET DEFAULT now();
ALTER TABLE activity_updates ALTER COLUMN created_at SET NOT NULL;

ALTER TABLE department_documents ADD COLUMN created_at TIMESTAMPTZ;
ALTER TABLE department_documents ADD COLUMN created_by UUID REFERENCES users(id);
ALTER TABLE department_documents ADD COLUMN creation_time_estimated BOOLEAN NOT NULL DEFAULT FALSE;
UPDATE department_documents d SET
 created_at=COALESCE((SELECT MIN(v.submitted_at) FROM department_document_versions v
                      WHERE v.department_document_id=d.id),d.submitted_at,now()),
 created_by=d.submitted_by,
 creation_time_estimated=TRUE;
ALTER TABLE department_documents ALTER COLUMN created_at SET DEFAULT now();
ALTER TABLE department_documents ALTER COLUMN created_at SET NOT NULL;
CREATE INDEX activity_updates_created_idx ON activity_updates(created_at);
CREATE INDEX department_documents_created_idx ON department_documents(created_at);
