-- Schema116: a start of a message in the protocol before the budget, which
-- the server refused because the Agent has an applied budget, gives its place
-- back at once. The server refuses every such start once this computer has
-- reported the budget (`shared_budget_execution_required`), and refuses it
-- again every time, since an applied budget stays applied. It looks for a
-- start of the message that it made before it reads the budget, so the
-- refusal also says that it made none. Until now such a start kept its place
-- until its message ended, a day at the latest, and the budget lane could
-- start neither that message nor the Agent's later ones after the same
-- process.
--
-- The refusal is kept, with its code and the digest of the request it
-- answered, and a release row beside it gives the place back. A release now
-- names the message it gives a place back for, not only its end, so the
-- release table is rebuilt without the reference to the end.
CREATE TABLE IF NOT EXISTS owner_shared_start_refusals (
 start_intent_id TEXT PRIMARY KEY REFERENCES owner_shared_start_intents(id),
 demand_id TEXT NOT NULL REFERENCES owner_shared_demands(id),
 code TEXT NOT NULL CHECK(code='shared_budget_execution_required'),
 input_digest TEXT NOT NULL CHECK(length(input_digest)=64),
 ledger_revision INTEGER NOT NULL UNIQUE CHECK(ledger_revision>0 AND ledger_revision<=9007199254740991)
) STRICT;
DROP TRIGGER IF EXISTS owner_shared_start_refusals_immutable;
CREATE TRIGGER owner_shared_start_refusals_immutable BEFORE UPDATE ON owner_shared_start_refusals BEGIN SELECT RAISE(ABORT,'immutable shared start refusal');END;
DROP TRIGGER IF EXISTS owner_shared_start_refusals_retained;
CREATE TRIGGER owner_shared_start_refusals_retained BEFORE DELETE ON owner_shared_start_refusals BEGIN SELECT RAISE(ABORT,'retained shared start refusal');END;
-- A refusal names one of its message's own starts, its first or a retry of
-- it, of the same Agent, for which this computer holds no command: no claim
-- and no fence.
DROP TRIGGER IF EXISTS owner_shared_start_refusals_proof;
CREATE TRIGGER owner_shared_start_refusals_proof BEFORE INSERT ON owner_shared_start_refusals
WHEN NOT EXISTS(SELECT 1 FROM owner_shared_demands o JOIN owner_shared_start_intents i ON i.child_id=o.child_id AND i.profile=o.profile
  WHERE o.id=NEW.demand_id AND i.id=NEW.start_intent_id
   AND (i.id IN('sdr1_'||o.id,'sdr2_'||o.id,'sdr3_'||o.id)
    OR i.id=(SELECT d.start_intent_id FROM owner_shared_demand_decisions d WHERE d.demand_id=o.id)))
 OR EXISTS(SELECT 1 FROM owner_shared_successor_attempts a WHERE a.start_intent_id=NEW.start_intent_id)
 OR EXISTS(SELECT 1 FROM owner_shared_successor_fences f WHERE f.start_intent_id=NEW.start_intent_id)
BEGIN SELECT RAISE(ABORT,'unproven shared start refusal');END;
-- Everything that names the release table goes before it is rebuilt.
DROP TRIGGER IF EXISTS owner_shared_start_intent_place;
DROP TRIGGER IF EXISTS owner_shared_start_releases_proof;
DROP VIEW IF EXISTS owner_shared_start_releasable;
CREATE TABLE owner_shared_start_releases_refusal_migration (
 start_intent_id TEXT PRIMARY KEY REFERENCES owner_shared_start_intents(id),
 demand_id TEXT NOT NULL REFERENCES owner_shared_demands(id)
) STRICT;
INSERT INTO owner_shared_start_releases_refusal_migration(start_intent_id,demand_id)
 SELECT start_intent_id,demand_id FROM owner_shared_start_releases;
DROP TABLE owner_shared_start_releases;
ALTER TABLE owner_shared_start_releases_refusal_migration RENAME TO owner_shared_start_releases;
CREATE TEMP TABLE start_refusal_migration_check(valid INTEGER NOT NULL CHECK(valid=1));
INSERT INTO start_refusal_migration_check SELECT NOT EXISTS(SELECT 1 FROM pragma_foreign_key_check('owner_shared_start_releases'));
DROP TABLE start_refusal_migration_check;
CREATE TRIGGER owner_shared_start_releases_immutable BEFORE UPDATE ON owner_shared_start_releases BEGIN SELECT RAISE(ABORT,'immutable shared start release');END;
CREATE TRIGGER owner_shared_start_releases_retained BEFORE DELETE ON owner_shared_start_releases BEGIN SELECT RAISE(ABORT,'retained shared start release');END;
-- Schema 115's view, without the starts already given back: a start refused
-- for the budget has given its place back before its message ends.
CREATE VIEW owner_shared_start_releasable AS
 SELECT i.id AS start_intent_id,t.demand_id AS demand_id
 FROM (SELECT demand_id,payload FROM owner_shared_budget_demand_terminals
  UNION ALL SELECT demand_id,payload FROM owner_shared_demand_second_terminals) t
 JOIN owner_shared_demands o ON o.id=t.demand_id
 JOIN owner_shared_start_intents i ON i.child_id=o.child_id AND i.profile=o.profile
  AND (i.id IN('sdr1_'||t.demand_id,'sdr2_'||t.demand_id,'sdr3_'||t.demand_id)
   OR i.id=(SELECT d.start_intent_id FROM owner_shared_demand_decisions d WHERE d.demand_id=t.demand_id))
 WHERE json_extract(t.payload,'$.version')=1
  AND (json_type(t.payload,'$.runtimeCommandId')='null'
   OR json_extract(t.payload,'$.runtimeCommandId')=json_extract(i.input_payload,'$.predecessor.commandId'))
  AND NOT EXISTS(SELECT 1 FROM owner_shared_successor_attempts a WHERE a.start_intent_id=i.id)
  AND NOT EXISTS(SELECT 1 FROM owner_shared_successor_fences f WHERE f.start_intent_id=i.id)
  AND NOT EXISTS(SELECT 1 FROM owner_shared_start_releases r WHERE r.start_intent_id=i.id);
-- A release names a message whose end gives the start back, or the refusal
-- of the start for the budget.
CREATE TRIGGER owner_shared_start_releases_proof BEFORE INSERT ON owner_shared_start_releases
WHEN NOT EXISTS(SELECT 1 FROM owner_shared_start_releasable v WHERE v.start_intent_id=NEW.start_intent_id AND v.demand_id=NEW.demand_id)
 AND NOT EXISTS(SELECT 1 FROM owner_shared_start_refusals f WHERE f.start_intent_id=NEW.start_intent_id AND f.demand_id=NEW.demand_id)
BEGIN SELECT RAISE(ABORT,'unproven shared start release');END;
-- Schema 114's place, unchanged.
CREATE TRIGGER owner_shared_start_intent_place BEFORE INSERT ON owner_shared_start_intents WHEN EXISTS(SELECT 1 FROM owner_shared_start_intents i WHERE i.child_id=NEW.child_id AND i.predecessor_digest=NEW.predecessor_digest AND COALESCE(i.period_id,'')=COALESCE(NEW.period_id,'') AND NOT EXISTS(SELECT 1 FROM owner_shared_start_releases r WHERE r.start_intent_id=i.id)) BEGIN SELECT RAISE(ABORT,'shared start place held');END;
