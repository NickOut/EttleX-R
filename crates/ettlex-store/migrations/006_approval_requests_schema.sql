-- Migration 006: Approval requests schema
-- Stores approval requests created by the route_for_approval policy.

PRAGMA strict = ON;

CREATE TABLE IF NOT EXISTS approval_requests (
    approval_token          TEXT PRIMARY KEY NOT NULL,
    reason_code             TEXT NOT NULL,
    candidate_set_json      TEXT NOT NULL,
    semantic_request_digest TEXT NOT NULL,
    status                  TEXT NOT NULL DEFAULT 'pending',
    created_at              INTEGER NOT NULL  -- milliseconds since epoch
);

CREATE INDEX IF NOT EXISTS idx_approval_requests_status ON approval_requests(status);
CREATE INDEX IF NOT EXISTS idx_approval_requests_created_at ON approval_requests(created_at);
