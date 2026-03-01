-- Migration 007: Add request_digest to approval_requests for CAS-backed payload storage.
--
-- This allows `approval.get` to reconstruct the full request payload from CAS
-- rather than reconstructing it from inline columns.

ALTER TABLE approval_requests ADD COLUMN request_digest TEXT;
CREATE INDEX IF NOT EXISTS idx_approval_requests_request_digest ON approval_requests(request_digest);
