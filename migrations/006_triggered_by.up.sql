-- Add triggered_by column to track who launched the agent (for TUI launches)
ALTER TABLE agents ADD COLUMN triggered_by TEXT;
