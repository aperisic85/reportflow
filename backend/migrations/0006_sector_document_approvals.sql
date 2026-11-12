-- Append-only internal document approval archive; the PDF bytes are retained.
-- Legacy locked documents remain 'locked' and are NOT falsely represented as approved.
ALTER TABLE sector_documents DROP CONSTRAINT IF EXISTS sector_documents_status_check;
ALTER TABLE sector_documents ADD CONSTRAINT sector_documents_status_check
 CHECK (status IN ('draft','locked','approved'));
CREATE TABLE sector_document_approvals (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
 sector_document_id UUID NOT NULL UNIQUE REFERENCES sector_documents(id) ON DELETE RESTRICT,
 verification_token UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
 document_number TEXT NOT NULL UNIQUE,
 approved_by UUID NOT NULL REFERENCES users(id),
 approved_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 approval_method TEXT NOT NULL DEFAULT 'internal_password_confirmation',
 pdf_format TEXT NOT NULL DEFAULT 'modern-letterhead-pdf',
 pdf_sha256 CHAR(64) NOT NULL,
 pdf_bytes BYTEA NOT NULL,
 pdf_size BIGINT NOT NULL CHECK(pdf_size > 0),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX sector_approvals_number_idx ON sector_document_approvals(document_number);
-- Prevent silently replacing or erasing archived approvals.
CREATE OR REPLACE FUNCTION prevent_approval_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 RAISE EXCEPTION 'Approved document archives are immutable';
END $$;
CREATE TRIGGER approvals_immutable_update BEFORE UPDATE OR DELETE ON sector_document_approvals
 FOR EACH ROW EXECUTE FUNCTION prevent_approval_mutation();
