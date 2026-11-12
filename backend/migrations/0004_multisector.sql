-- Multi-sector organization, preserving the existing 'admin' as Mega admin.
CREATE TABLE sectors (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
 code TEXT NOT NULL UNIQUE,
 name TEXT NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO sectors(code,name) VALUES ('SP','Sigurnost plovidbe');
ALTER TABLE departments ADD COLUMN sector_id UUID REFERENCES sectors(id);
UPDATE departments SET sector_id=(SELECT id FROM sectors WHERE code='SP');
ALTER TABLE departments ALTER COLUMN sector_id SET NOT NULL;
ALTER TABLE departments DROP CONSTRAINT IF EXISTS departments_code_key;
CREATE UNIQUE INDEX departments_sector_code_unique ON departments(sector_id,code);
CREATE INDEX departments_sector_idx ON departments(sector_id,sort_order);

ALTER TABLE users DROP CONSTRAINT IF EXISTS users_role_check;
ALTER TABLE users ADD CONSTRAINT users_role_check CHECK (role IN ('admin','org_admin','director','manager','employee'));
ALTER TABLE users ADD COLUMN sector_id UUID REFERENCES sectors(id);
UPDATE users SET sector_id=(SELECT d.sector_id FROM departments d WHERE d.id=users.department_id)
 WHERE department_id IS NOT NULL;
UPDATE users SET sector_id=(SELECT id FROM sectors WHERE code='SP') WHERE role='director' AND sector_id IS NULL;
UPDATE users SET department_id=NULL,sector_id=NULL WHERE role='admin';
UPDATE users SET department_id=NULL WHERE role='director';
CREATE INDEX users_sector_idx ON users(sector_id);

ALTER TABLE sector_documents ADD COLUMN sector_id UUID REFERENCES sectors(id);
UPDATE sector_documents SET sector_id=(SELECT id FROM sectors WHERE code='SP');
ALTER TABLE sector_documents ALTER COLUMN sector_id SET NOT NULL;
ALTER TABLE sector_documents DROP CONSTRAINT IF EXISTS sector_documents_year_month_kind_revision_key;
CREATE UNIQUE INDEX sector_documents_sector_revision_unique
 ON sector_documents(sector_id,year,month,kind,revision);
CREATE INDEX sector_documents_sector_lookup_idx ON sector_documents(sector_id,year,month,kind);

-- Validate user scope before enabling new report workflows.
ALTER TABLE users ADD CONSTRAINT users_scope_valid CHECK (
  (role IN ('admin','org_admin') AND department_id IS NULL AND sector_id IS NULL)
  OR (role='director' AND department_id IS NULL AND sector_id IS NOT NULL)
  OR (role IN ('manager','employee') AND department_id IS NOT NULL AND sector_id IS NOT NULL)
);
