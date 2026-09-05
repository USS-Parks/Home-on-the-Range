CREATE TABLE mutation_audit (
    sequence INTEGER PRIMARY KEY,
    principal TEXT NOT NULL CHECK(length(CAST(principal AS BLOB)) BETWEEN 1 AND 128),
    namespace TEXT NOT NULL,
    record_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    operation TEXT NOT NULL CHECK(operation IN ('create','revise')),
    committed_at_ms INTEGER NOT NULL CHECK(committed_at_ms >= 0),
    FOREIGN KEY(namespace,record_id,revision) REFERENCES revisions(namespace,record_id,revision)
) STRICT;
CREATE TABLE write_receipts (
    principal TEXT NOT NULL CHECK(length(CAST(principal AS BLOB)) BETWEEN 1 AND 128),
    idempotency_key TEXT NOT NULL CHECK(length(CAST(idempotency_key AS BLOB)) BETWEEN 1 AND 128),
    request_hash BLOB NOT NULL CHECK(length(request_hash)=32),
    namespace TEXT NOT NULL,
    record_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    committed_at_ms INTEGER NOT NULL CHECK(committed_at_ms >= 0),
    audit_sequence INTEGER NOT NULL UNIQUE REFERENCES mutation_audit(sequence),
    PRIMARY KEY(principal,idempotency_key),
    FOREIGN KEY(namespace,record_id,revision) REFERENCES revisions(namespace,record_id,revision)
) STRICT;
CREATE TRIGGER receipts_no_update BEFORE UPDATE ON write_receipts BEGIN SELECT RAISE(ABORT,'immutable receipt'); END;
CREATE TRIGGER receipts_no_delete BEFORE DELETE ON write_receipts BEGIN SELECT RAISE(ABORT,'immutable receipt'); END;
CREATE TRIGGER mutation_audit_no_update BEFORE UPDATE ON mutation_audit BEGIN SELECT RAISE(ABORT,'immutable audit'); END;
CREATE TRIGGER mutation_audit_no_delete BEFORE DELETE ON mutation_audit BEGIN SELECT RAISE(ABORT,'immutable audit'); END;
PRAGMA user_version=3;
