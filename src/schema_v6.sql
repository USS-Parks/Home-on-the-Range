-- Atomic owner import receipts are encrypted with the rest of the vault.
CREATE TABLE import_identity(id INTEGER PRIMARY KEY CHECK(id=1), nonce BLOB NOT NULL CHECK(length(nonce)=32)) STRICT;
INSERT INTO import_identity VALUES(1,randomblob(32));
CREATE TRIGGER import_identity_immutable_update BEFORE UPDATE ON import_identity
BEGIN SELECT RAISE(ABORT,'import identity is immutable'); END;
CREATE TRIGGER import_identity_immutable_delete BEFORE DELETE ON import_identity
BEGIN SELECT RAISE(ABORT,'import identity is immutable'); END;
CREATE TABLE import_receipts(
    preview_digest TEXT PRIMARY KEY CHECK(length(preview_digest)=64),
    batch_hash TEXT NOT NULL CHECK(length(batch_hash)=64),
    result_json TEXT NOT NULL,
    committed_at_ms INTEGER NOT NULL
) STRICT;
CREATE TRIGGER import_receipts_immutable_update BEFORE UPDATE ON import_receipts
BEGIN SELECT RAISE(ABORT,'import receipts are immutable'); END;
CREATE TRIGGER import_receipts_immutable_delete BEFORE DELETE ON import_receipts
BEGIN SELECT RAISE(ABORT,'import receipts are immutable'); END;
