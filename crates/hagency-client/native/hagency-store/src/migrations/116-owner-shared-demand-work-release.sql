-- ADR-193 amendment (retry v2): when a claimed demand attempt stops before it
-- began its demand's work, the binding moves, byte for byte, into a retained
-- release row and the demand waits again. The released work is never bound
-- or admitted again; a binding row still never changes in place. The server
-- gives back only the work of a start stored with `reoffers`, and a start is
-- always sent with its first bytes: the event's original start saves it, a
-- retry fixes it when first prepared, and a lapse row, saved when a retry of
-- an event that opted in is first prepared without it, ends that for the event.
CREATE TABLE owner_shared_demand_reoffer_lapses (
 demand_id TEXT PRIMARY KEY REFERENCES owner_shared_demands(id),
 ledger_revision INTEGER NOT NULL UNIQUE CHECK(ledger_revision>0 AND ledger_revision<=9007199254740991)
) STRICT;
CREATE TRIGGER owner_shared_demand_reoffer_lapses_immutable BEFORE UPDATE ON owner_shared_demand_reoffer_lapses BEGIN SELECT RAISE(ABORT,'immutable reoffer lapse');END;
CREATE TRIGGER owner_shared_demand_reoffer_lapses_retained BEFORE DELETE ON owner_shared_demand_reoffer_lapses BEGIN SELECT RAISE(ABORT,'retained reoffer lapse');END;
CREATE TABLE owner_shared_demand_released_bindings (
 work_id TEXT PRIMARY KEY,
 demand_id TEXT NOT NULL REFERENCES owner_shared_demands(id),
 binding_payload TEXT NOT NULL,
 binding_digest TEXT NOT NULL CHECK(length(binding_digest)=64),
 ledger_revision INTEGER NOT NULL,
 released_revision INTEGER NOT NULL UNIQUE CHECK(released_revision>0 AND released_revision<=9007199254740991),
 released_at_ms INTEGER NOT NULL CHECK(released_at_ms>0 AND released_at_ms<=9007199254740991)
) STRICT;
CREATE INDEX owner_shared_demand_released_binding_demand ON owner_shared_demand_released_bindings(demand_id);
CREATE TRIGGER owner_shared_demand_released_bindings_immutable BEFORE UPDATE ON owner_shared_demand_released_bindings BEGIN SELECT RAISE(ABORT,'immutable released demand binding');END;
CREATE TRIGGER owner_shared_demand_released_bindings_retained BEFORE DELETE ON owner_shared_demand_released_bindings BEGIN SELECT RAISE(ABORT,'retained released demand binding');END;
-- A copy of the exact current binding of an event that opted in, whose
-- attempt has stopped and whose work is provably unbegun: no start, call,
-- tool or reply event, no work observation, no reply delivery and no call
-- row of any state.
CREATE TRIGGER owner_shared_demand_released_bindings_unbegun BEFORE INSERT ON owner_shared_demand_released_bindings
WHEN NOT EXISTS(SELECT 1 FROM owner_shared_demand_bindings b WHERE b.demand_id=NEW.demand_id AND b.work_id=NEW.work_id AND b.binding_payload=NEW.binding_payload AND b.binding_digest=NEW.binding_digest AND b.ledger_revision=NEW.ledger_revision)
 OR NOT EXISTS(SELECT 1 FROM owner_shared_demand_decisions x WHERE x.demand_id=NEW.demand_id AND json_extract(x.input_payload,'$.reoffers')=1)
 OR EXISTS(SELECT 1 FROM owner_shared_demand_reoffer_lapses l WHERE l.demand_id=NEW.demand_id)
 OR NOT EXISTS(SELECT 1 FROM owner_shared_successor_observations o WHERE o.attempt_id=json_extract(NEW.binding_payload,'$.runtimeCommandId') AND json_extract(o.payload,'$.state')='stopped')
 OR EXISTS(SELECT 1 FROM owner_shared_work_events WHERE work_id=NEW.work_id)
 OR EXISTS(SELECT 1 FROM owner_shared_work_observations WHERE work_id=NEW.work_id)
 OR EXISTS(SELECT 1 FROM owner_shared_reply_deliveries WHERE work_id=NEW.work_id)
 OR EXISTS(SELECT 1 FROM owner_shared_successor_call_pins WHERE work_id=NEW.work_id)
 OR EXISTS(SELECT 1 FROM owner_managed_calls c JOIN owner_shared_work_inbox w ON w.execution_id=c.execution_id AND w.child_id=c.child_id WHERE w.id=NEW.work_id)
BEGIN SELECT RAISE(ABORT,'unbegun demand work required');END;
-- How a claimed attempt's stop reaches the server, decided once when its
-- stop is recorded: 'retry' asks for retry `retry` of `demand_id` instead of
-- the plain result upload, on the first upload and on every resend.
CREATE TABLE owner_shared_stop_routes (
 attempt_id TEXT PRIMARY KEY REFERENCES owner_shared_successor_attempts(id),
 child_id TEXT NOT NULL,
 profile TEXT NOT NULL CHECK(length(profile)=64),
 observation_id TEXT NOT NULL REFERENCES owner_shared_successor_observations(id),
 route TEXT NOT NULL CHECK(route IN('plain','retry')),
 demand_id TEXT REFERENCES owner_shared_demands(id),
 retry INTEGER CHECK(retry BETWEEN 1 AND 4),
 released_work_id TEXT UNIQUE REFERENCES owner_shared_demand_released_bindings(work_id),
 ledger_revision INTEGER NOT NULL UNIQUE CHECK(ledger_revision>0 AND ledger_revision<=9007199254740991),
 CHECK((route='retry')=(demand_id IS NOT NULL) AND (route='retry')=(retry IS NOT NULL) AND (route='retry' OR released_work_id IS NULL))
) STRICT;
CREATE INDEX owner_shared_stop_route_demand ON owner_shared_stop_routes(demand_id);
CREATE TRIGGER owner_shared_stop_routes_immutable BEFORE UPDATE ON owner_shared_stop_routes BEGIN SELECT RAISE(ABORT,'immutable stop route');END;
CREATE TRIGGER owner_shared_stop_routes_retained BEFORE DELETE ON owner_shared_stop_routes BEGIN SELECT RAISE(ABORT,'retained stop route');END;
CREATE TRIGGER owner_shared_stop_routes_proof BEFORE INSERT ON owner_shared_stop_routes
WHEN NOT EXISTS(SELECT 1 FROM owner_shared_successor_observations o JOIN owner_shared_successor_attempts a ON a.id=o.attempt_id WHERE o.id=NEW.observation_id AND o.attempt_id=NEW.attempt_id AND a.child_id=NEW.child_id AND a.profile=NEW.profile AND json_extract(o.payload,'$.state')='stopped')
 OR NEW.route='retry' AND (EXISTS(SELECT 1 FROM owner_shared_demand_bindings WHERE demand_id=NEW.demand_id) OR EXISTS(SELECT 1 FROM owner_shared_budget_demand_terminals WHERE demand_id=NEW.demand_id) OR NOT EXISTS(SELECT 1 FROM owner_shared_demand_decisions x WHERE x.demand_id=NEW.demand_id AND json_extract(x.input_payload,'$.reoffers')=1) OR EXISTS(SELECT 1 FROM owner_shared_demand_reoffer_lapses l WHERE l.demand_id=NEW.demand_id))
 OR NEW.released_work_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM owner_shared_demand_released_bindings r WHERE r.work_id=NEW.released_work_id AND r.demand_id=NEW.demand_id AND json_extract(r.binding_payload,'$.runtimeCommandId')=NEW.attempt_id)
BEGIN SELECT RAISE(ABORT,'stop route proof');END;
-- Bindings stay immutable. A row leaves only beside its exact release, and
-- released work, or work its stopped attempt handed back, is never bound again.
DROP TRIGGER IF EXISTS owner_shared_demand_bindings_retained;
CREATE TRIGGER owner_shared_demand_bindings_retained BEFORE DELETE ON owner_shared_demand_bindings
WHEN NOT EXISTS(SELECT 1 FROM owner_shared_demand_released_bindings r WHERE r.work_id=OLD.work_id AND r.demand_id=OLD.demand_id AND r.binding_payload=OLD.binding_payload AND r.binding_digest=OLD.binding_digest AND r.ledger_revision=OLD.ledger_revision)
BEGIN SELECT RAISE(ABORT,'retained demand work binding');END;
DROP TRIGGER IF EXISTS owner_shared_demand_bindings_released;
CREATE TRIGGER owner_shared_demand_bindings_released BEFORE INSERT ON owner_shared_demand_bindings
WHEN EXISTS(SELECT 1 FROM owner_shared_demand_released_bindings WHERE work_id=NEW.work_id)
 OR EXISTS(SELECT 1 FROM owner_shared_stop_routes WHERE attempt_id=json_extract(NEW.binding_payload,'$.runtimeCommandId') AND route='retry' AND demand_id=NEW.demand_id)
BEGIN SELECT RAISE(ABORT,'released demand work');END;
