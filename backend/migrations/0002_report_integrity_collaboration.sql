-- Immutable snapshots for every department submission; existing submitted records get
-- a baseline version with the data available at migration time.
CREATE TABLE department_document_versions (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
 department_document_id UUID NOT NULL REFERENCES department_documents(id),
 version INT NOT NULL CHECK (version>0),
 snapshot JSONB NOT NULL,
 submitted_by UUID NOT NULL REFERENCES users(id),
 submitted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 UNIQUE(department_document_id,version)
);
CREATE INDEX dept_versions_doc_idx ON department_document_versions(department_document_id,version DESC);
INSERT INTO department_document_versions(department_document_id,version,snapshot,submitted_by,submitted_at)
SELECT dd.id,1,jsonb_build_object('year',dd.year,'month',dd.month,'kind',dd.kind,'department_id',dd.department_id,'items',COALESCE((
 SELECT jsonb_agg(jsonb_build_object('id',a.id,'title',a.title,'description',a.description,'parent_id',a.parent_id,
 'type',a.type,'status',a.status,'content',COALESCE(u.content,''),'progress',COALESCE(u.progress,'in_progress'))
 ORDER BY a.sort_order,a.created_at)
 FROM activities a LEFT JOIN activity_updates u ON u.activity_id=a.id AND u.year=dd.year AND u.month=dd.month AND u.kind=dd.kind
 WHERE a.department_id=dd.department_id AND COALESCE(u.include_in_document,true)
),'[]'::jsonb)),dd.submitted_by,COALESCE(dd.submitted_at,now())
FROM department_documents dd WHERE dd.status='submitted' AND dd.submitted_by IS NOT NULL;

CREATE TABLE activity_threads (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
 activity_id UUID NOT NULL REFERENCES activities(id) ON DELETE CASCADE,
 title TEXT NOT NULL CHECK (char_length(title) BETWEEN 1 AND 250),
 status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open','done')),
 created_by UUID NOT NULL REFERENCES users(id),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX threads_activity_idx ON activity_threads(activity_id,updated_at DESC);
CREATE TABLE activity_thread_participants (
 thread_id UUID NOT NULL REFERENCES activity_threads(id) ON DELETE CASCADE,
 user_id UUID NOT NULL REFERENCES users(id),
 added_by UUID REFERENCES users(id),
 added_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 PRIMARY KEY(thread_id,user_id)
);
CREATE TABLE activity_thread_messages (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
 thread_id UUID NOT NULL REFERENCES activity_threads(id) ON DELETE CASCADE,
 author_id UUID NOT NULL REFERENCES users(id),
 body TEXT NOT NULL CHECK(char_length(body) BETWEEN 1 AND 12000),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX thread_messages_order_idx ON activity_thread_messages(thread_id,created_at,id);
CREATE TABLE activity_thread_events (
 id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 thread_id UUID NOT NULL REFERENCES activity_threads(id) ON DELETE CASCADE,
 actor_id UUID NOT NULL REFERENCES users(id),
 event_type TEXT NOT NULL,
 payload JSONB NOT NULL DEFAULT '{}'::jsonb,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX thread_events_idx ON activity_thread_events(thread_id,created_at DESC);

-- Per-user read state supports an inbox without materializing one notification per message.
ALTER TABLE activity_thread_participants ADD COLUMN last_read_at TIMESTAMPTZ NOT NULL DEFAULT '-infinity'::timestamptz;
CREATE INDEX thread_messages_unread_idx ON activity_thread_messages(thread_id,created_at DESC);

-- Every monthly-plan/report edit is retained as an immutable event; old rows
-- receive a baseline copy using their last known timestamp.
CREATE TABLE activity_update_revisions (
 id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 update_id UUID NOT NULL REFERENCES activity_updates(id),
 activity_id UUID NOT NULL REFERENCES activities(id),
 year INT NOT NULL,
 month INT NOT NULL,
 kind TEXT NOT NULL,
 content TEXT NOT NULL,
 progress TEXT NOT NULL,
 include_in_document BOOLEAN NOT NULL,
 author_id UUID REFERENCES users(id),
 changed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO activity_update_revisions(update_id,activity_id,year,month,kind,content,progress,include_in_document,author_id,changed_at)
 SELECT id,activity_id,year,month,kind,content,progress,include_in_document,updated_by,updated_at FROM activity_updates;
CREATE OR REPLACE FUNCTION record_activity_update_revision() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 INSERT INTO activity_update_revisions(update_id,activity_id,year,month,kind,content,progress,include_in_document,author_id,changed_at)
 VALUES(NEW.id,NEW.activity_id,NEW.year,NEW.month,NEW.kind,NEW.content,NEW.progress,NEW.include_in_document,NEW.updated_by,NEW.updated_at);
 RETURN NEW;
END $$;
CREATE TRIGGER activity_update_revision_capture AFTER INSERT OR UPDATE OF content,progress,include_in_document,updated_by,updated_at
ON activity_updates FOR EACH ROW EXECUTE FUNCTION record_activity_update_revision();
CREATE INDEX update_revisions_idx ON activity_update_revisions(activity_id,year,month,kind,changed_at DESC);

CREATE TABLE sector_document_edits (
 id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 sector_document_id UUID NOT NULL REFERENCES sector_documents(id),
 snapshot JSONB NOT NULL,
 introduction TEXT NOT NULL,
 conclusion TEXT NOT NULL,
 edited_by UUID NOT NULL REFERENCES users(id),
 edited_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX sector_document_edits_idx ON sector_document_edits(sector_document_id,edited_at DESC);
