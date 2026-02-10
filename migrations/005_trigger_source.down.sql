-- Remove trigger_source column from agents table
ALTER TABLE agents DROP COLUMN trigger_source;

-- Drop trigger_source enum
DROP TYPE trigger_source;
