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
