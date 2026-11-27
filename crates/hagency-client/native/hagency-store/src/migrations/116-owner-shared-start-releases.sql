-- A saved start intent holds the place after its predecessor in its period,
-- so a process is never asked to start twice. Schema 109 made that place a
-- unique index, held for good: a start the server refused, of an event the
-- server then ended, kept every later start of that predecessor waiting until
-- the period turned. The server makes no start of an ended event, so such a
-- start gives its place back with a retained release row, written beside the
-- event's end. At most one start intent without a release holds each place.
CREATE TABLE IF NOT EXISTS owner_shared_start_releases (
 start_intent_id TEXT PRIMARY KEY REFERENCES owner_shared_start_intents(id),
 demand_id TEXT NOT NULL REFERENCES owner_shared_budget_demand_terminals(demand_id)
) STRICT;
DROP TRIGGER IF EXISTS owner_shared_start_releases_immutable;
CREATE TRIGGER owner_shared_start_releases_immutable BEFORE UPDATE ON owner_shared_start_releases BEGIN SELECT RAISE(ABORT,'immutable shared start release');END;
DROP TRIGGER IF EXISTS owner_shared_start_releases_retained;
CREATE TRIGGER owner_shared_start_releases_retained BEFORE DELETE ON owner_shared_start_releases BEGIN SELECT RAISE(ABORT,'retained shared start release');END;
-- The starts an ended event gives back: its first start or a retry of it,
-- when the end (written from the server's status) names no start of the
-- event, or names the start this one followed, and when no command was
-- issued here for it. The server's status names the event's latest start.
DROP VIEW IF EXISTS owner_shared_start_releasable;
CREATE VIEW owner_shared_start_releasable AS
 SELECT i.id AS start_intent_id,t.demand_id AS demand_id
 FROM owner_shared_budget_demand_terminals t
 JOIN owner_shared_demands o ON o.id=t.demand_id
 JOIN owner_shared_start_intents i ON i.child_id=o.child_id AND i.profile=o.profile
  AND (i.id IN('sdr1_'||t.demand_id,'sdr2_'||t.demand_id,'sdr3_'||t.demand_id)
   OR i.id=(SELECT d.start_intent_id FROM owner_shared_demand_decisions d WHERE d.demand_id=t.demand_id))
 WHERE json_extract(t.payload,'$.version')=1
  AND (json_type(t.payload,'$.runtimeCommandId')='null'
   OR json_extract(t.payload,'$.runtimeCommandId')=json_extract(i.input_payload,'$.predecessor.commandId'))
  AND NOT EXISTS(SELECT 1 FROM owner_shared_successor_attempts a WHERE a.start_intent_id=i.id)
  AND NOT EXISTS(SELECT 1 FROM owner_shared_successor_fences f WHERE f.start_intent_id=i.id);
DROP TRIGGER IF EXISTS owner_shared_start_releases_proof;
CREATE TRIGGER owner_shared_start_releases_proof BEFORE INSERT ON owner_shared_start_releases WHEN NOT EXISTS(SELECT 1 FROM owner_shared_start_releasable v WHERE v.start_intent_id=NEW.start_intent_id AND v.demand_id=NEW.demand_id) BEGIN SELECT RAISE(ABORT,'unproven shared start release');END;
-- The same place as the schema-109 index, now held only without a release.
DROP INDEX IF EXISTS owner_shared_start_intent_period;
CREATE INDEX IF NOT EXISTS owner_shared_start_intent_predecessor ON owner_shared_start_intents(child_id,predecessor_digest,COALESCE(period_id,''));
DROP TRIGGER IF EXISTS owner_shared_start_intent_place;
CREATE TRIGGER owner_shared_start_intent_place BEFORE INSERT ON owner_shared_start_intents WHEN EXISTS(SELECT 1 FROM owner_shared_start_intents i WHERE i.child_id=NEW.child_id AND i.predecessor_digest=NEW.predecessor_digest AND COALESCE(i.period_id,'')=COALESCE(NEW.period_id,'') AND NOT EXISTS(SELECT 1 FROM owner_shared_start_releases r WHERE r.start_intent_id=i.id)) BEGIN SELECT RAISE(ABORT,'shared start place held');END;
-- Ends retained before this schema give their places back now.
INSERT OR IGNORE INTO owner_shared_start_releases(start_intent_id,demand_id) SELECT start_intent_id,demand_id FROM owner_shared_start_releasable;
