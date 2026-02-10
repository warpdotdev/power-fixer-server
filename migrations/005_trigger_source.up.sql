-- Add trigger_source enum to track where agents are initiated from
CREATE TYPE trigger_source AS ENUM ('tui', 'github_webhook');

-- Add trigger_source column to agents table (default 'tui' for backwards compatibility)
ALTER TABLE agents ADD COLUMN trigger_source trigger_source NOT NULL DEFAULT 'tui';
