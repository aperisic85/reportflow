CREATE TABLE unplanned_activity_entries (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
 department_id UUID NOT NULL REFERENCES departments(id),
 performed_on DATE NOT NULL,
 title TEXT NOT NULL CHECK (char_length(title) BETWEEN 1 AND 250),
 description TEXT NOT NULL CHECK (char_length(description) BETWEEN 1 AND 12000),
 related_activity_id UUID REFERENCES activities(id) ON DELETE SET NULL,
 created_by UUID REFERENCES users(id),
 updated_by UUID REFERENCES users(id),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX unplanned_entries_period_idx
 ON unplanned_activity_entries(department_id,performed_on,created_at);
CREATE INDEX unplanned_entries_activity_idx
 ON unplanned_activity_entries(related_activity_id)
 WHERE related_activity_id IS NOT NULL;
