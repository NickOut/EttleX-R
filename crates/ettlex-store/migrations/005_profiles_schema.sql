-- Migration 005: Profiles schema
-- Stores named profiles with ambiguity_policy and other config.

PRAGMA strict = ON;

CREATE TABLE IF NOT EXISTS profiles (
    profile_ref  TEXT PRIMARY KEY NOT NULL,
    payload_json TEXT NOT NULL,
    is_default   INTEGER NOT NULL DEFAULT 0,
    created_at   INTEGER NOT NULL   -- milliseconds since epoch
);

CREATE INDEX IF NOT EXISTS idx_profiles_is_default ON profiles(is_default);
