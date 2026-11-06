CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE TABLE departments (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  code TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  sort_order INT NOT NULL DEFAULT 0
);
CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  username TEXT NOT NULL UNIQUE,
  full_name TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('admin','director','manager','employee')),
  department_id UUID REFERENCES departments(id),
  active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE activities (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  department_id UUID NOT NULL REFERENCES departments(id),
  parent_id UUID REFERENCES activities(id),
  title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  type TEXT NOT NULL DEFAULT 'ongoing' CHECK (type IN ('ongoing','project','one_off')),
  status TEXT NOT NULL DEFAULT 'in_progress' CHECK (status IN ('in_progress','completed','paused','cancelled')),
  sort_order INT NOT NULL DEFAULT 0,
  created_by UUID REFERENCES users(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX activities_department_idx ON activities(department_id,sort_order);
CREATE TABLE activity_updates (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  activity_id UUID NOT NULL REFERENCES activities(id) ON DELETE CASCADE,
  year INT NOT NULL CHECK (year BETWEEN 2000 AND 2100),
  month INT NOT NULL CHECK (month BETWEEN 1 AND 12),
  kind TEXT NOT NULL CHECK (kind IN ('plan','report')),
  content TEXT NOT NULL,
  progress TEXT NOT NULL DEFAULT 'in_progress' CHECK (progress IN ('in_progress','completed','postponed','not_started')),
  include_in_document BOOLEAN NOT NULL DEFAULT TRUE,
  updated_by UUID REFERENCES users(id),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(activity_id,year,month,kind)
);
CREATE TABLE department_documents (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  department_id UUID NOT NULL REFERENCES departments(id),
  year INT NOT NULL,
  month INT NOT NULL CHECK (month BETWEEN 1 AND 12),
  kind TEXT NOT NULL CHECK (kind IN ('plan','report')),
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','submitted')),
  submitted_by UUID REFERENCES users(id),
  submitted_at TIMESTAMPTZ,
  UNIQUE(department_id,year,month,kind)
);
CREATE TABLE sector_documents (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  year INT NOT NULL,
  month INT NOT NULL CHECK (month BETWEEN 1 AND 12),
  kind TEXT NOT NULL CHECK (kind IN ('plan','report')),
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','locked')),
  title TEXT NOT NULL,
  introduction TEXT NOT NULL DEFAULT '',
  conclusion TEXT NOT NULL DEFAULT '',
  snapshot JSONB NOT NULL,
  revision INT NOT NULL DEFAULT 1,
  created_by UUID REFERENCES users(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  locked_at TIMESTAMPTZ,
  UNIQUE(year,month,kind,revision)
);
CREATE TABLE audit_events (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  user_id UUID REFERENCES users(id),
  action TEXT NOT NULL,
  entity_id UUID,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO departments(code,name,sort_order) VALUES
 ('RE','Radio-elektrotehnički poslovi',1),
 ('HG','Hidro-građevinski poslovi',2),
 ('BS','Brodostrojarski poslovi',3),
 ('NA','Nautički poslovi',4);
