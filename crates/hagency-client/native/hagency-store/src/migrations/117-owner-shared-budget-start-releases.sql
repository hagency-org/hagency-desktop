-- Schema115: the budget lane gives back the place of a start the server never
-- made, as schema 114 does for the plain lane. A saved budget start held the
-- place after its predecessor for good, as a unique pair of the table: a start
-- the server refused, of an event the server then ended, kept every later
-- start of that predecessor waiting for as long as the Agent's reservation
-- lived. The server makes no start of an ended event, and the event's status
-- names its start when the server made one, so such a start gives its place
-- back with a retained release row, written beside the event's end. The table
-- is rebuilt without the pair; at most one budget start intent without a
-- release holds each place.
DROP TRIGGER IF EXISTS owner_shared_budget_start_releases_proof;
DROP VIEW IF EXISTS owner_shared_budget_start_releasable;
PRAGMA defer_foreign_keys=ON;
CREATE TABLE owner_shared_budget_start_intents_release_migration (
 id TEXT PRIMARY KEY,
 child_id TEXT NOT NULL REFERENCES owner_shared_runtime_lifetimes(child_id),
 profile TEXT NOT NULL CHECK(length(profile)=64),
 device_id TEXT NOT NULL,
 device_generation INTEGER NOT NULL CHECK(device_generation>0 AND device_generation<=9007199254740991),
 predecessor_digest TEXT NOT NULL CHECK(length(predecessor_digest)=64),
 original_runtime_digest TEXT NOT NULL CHECK(length(original_runtime_digest)=64),
 original_runtime_payload TEXT NOT NULL CHECK(json_valid(original_runtime_payload) AND length(CAST(original_runtime_payload AS BLOB))<=16384),
 input_payload TEXT NOT NULL CHECK(json_valid(input_payload) AND length(CAST(input_payload AS BLOB))<=16384),
 input_digest TEXT NOT NULL CHECK(length(input_digest)=64),
 ledger_revision INTEGER NOT NULL UNIQUE CHECK(ledger_revision>0 AND ledger_revision<=9007199254740991)
) STRICT;
INSERT INTO owner_shared_budget_start_intents_release_migration(id,child_id,profile,device_id,device_generation,predecessor_digest,original_runtime_digest,original_runtime_payload,input_payload,input_digest,ledger_revision)
 SELECT id,child_id,profile,device_id,device_generation,predecessor_digest,original_runtime_digest,original_runtime_payload,input_payload,input_digest,ledger_revision FROM owner_shared_budget_start_intents;
DROP TABLE owner_shared_budget_start_intents;
ALTER TABLE owner_shared_budget_start_intents_release_migration RENAME TO owner_shared_budget_start_intents;
CREATE INDEX owner_shared_budget_start_intent_predecessor ON owner_shared_budget_start_intents(child_id,predecessor_digest);
CREATE TRIGGER owner_shared_budget_start_intents_immutable BEFORE UPDATE ON owner_shared_budget_start_intents BEGIN SELECT RAISE(ABORT,'immutable budget start intent'); END;
CREATE TRIGGER owner_shared_budget_start_intents_retained BEFORE DELETE ON owner_shared_budget_start_intents BEGIN SELECT RAISE(ABORT,'retained budget start intent'); END;
CREATE TEMP TABLE budget_start_release_migration_check(valid INTEGER NOT NULL CHECK(valid=1));
INSERT INTO budget_start_release_migration_check SELECT NOT EXISTS(SELECT 1 FROM pragma_foreign_key_check);
DROP TABLE budget_start_release_migration_check;
PRAGMA defer_foreign_keys=OFF;
CREATE TABLE IF NOT EXISTS owner_shared_budget_start_releases (
 start_intent_id TEXT PRIMARY KEY REFERENCES owner_shared_budget_start_intents(id),
 demand_id TEXT NOT NULL REFERENCES owner_shared_budget_demand_terminals(demand_id)
) STRICT;
DROP TRIGGER IF EXISTS owner_shared_budget_start_releases_immutable;
CREATE TRIGGER owner_shared_budget_start_releases_immutable BEFORE UPDATE ON owner_shared_budget_start_releases BEGIN SELECT RAISE(ABORT,'immutable budget start release');END;
DROP TRIGGER IF EXISTS owner_shared_budget_start_releases_retained;
CREATE TRIGGER owner_shared_budget_start_releases_retained BEFORE DELETE ON owner_shared_budget_start_releases BEGIN SELECT RAISE(ABORT,'retained budget start release');END;
-- An end read from the budget lane's status (no version, an execution budget)
-- that names no start of the event; its start, of the same Agent, for which
-- this computer holds no claim and no fence.
CREATE VIEW owner_shared_budget_start_releasable AS
 SELECT i.id AS start_intent_id,t.demand_id AS demand_id
 FROM owner_shared_budget_demand_terminals t
 JOIN owner_shared_demands o ON o.id=t.demand_id
 JOIN owner_shared_budget_demand_decisions d ON d.demand_id=t.demand_id
 JOIN owner_shared_budget_start_intents i ON i.id=d.start_intent_id AND i.child_id=o.child_id AND i.profile=o.profile
 WHERE json_type(t.payload,'$.version') IS NULL
  AND json_type(t.payload,'$.executionBudget')='object'
  AND json_type(t.payload,'$.runtimeCommandId')='null'
  AND NOT EXISTS(SELECT 1 FROM owner_shared_successor_attempts a WHERE a.start_intent_id=i.id)
  AND NOT EXISTS(SELECT 1 FROM owner_shared_successor_fences f WHERE f.start_intent_id=i.id);
CREATE TRIGGER owner_shared_budget_start_releases_proof BEFORE INSERT ON owner_shared_budget_start_releases WHEN NOT EXISTS(SELECT 1 FROM owner_shared_budget_start_releasable v WHERE v.start_intent_id=NEW.start_intent_id AND v.demand_id=NEW.demand_id) BEGIN SELECT RAISE(ABORT,'unproven budget start release');END;
CREATE TRIGGER owner_shared_budget_start_intent_place BEFORE INSERT ON owner_shared_budget_start_intents WHEN EXISTS(SELECT 1 FROM owner_shared_budget_start_intents i WHERE i.child_id=NEW.child_id AND i.predecessor_digest=NEW.predecessor_digest AND NOT EXISTS(SELECT 1 FROM owner_shared_budget_start_releases r WHERE r.start_intent_id=i.id)) BEGIN SELECT RAISE(ABORT,'budget start place held');END;
INSERT OR IGNORE INTO owner_shared_budget_start_releases(start_intent_id,demand_id) SELECT start_intent_id,demand_id FROM owner_shared_budget_start_releasable;
