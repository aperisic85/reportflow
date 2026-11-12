-- Audit changes of permanent activity fields. Do not modify earlier migrations.
CREATE TABLE activity_field_changes (
 id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 activity_id UUID NOT NULL REFERENCES activities(id),
 changed_by UUID NOT NULL REFERENCES users(id),
 old_title TEXT NOT NULL,
 new_title TEXT NOT NULL,
 old_description TEXT NOT NULL,
 new_description TEXT NOT NULL,
 changed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX activity_field_changes_history_idx ON activity_field_changes(activity_id,changed_at DESC,id DESC);
