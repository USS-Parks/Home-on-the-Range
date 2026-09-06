ALTER TABLE record_visibility ADD COLUMN valid_from_ms INTEGER CHECK(valid_from_ms IS NULL OR valid_from_ms>=0);
ALTER TABLE clients ADD COLUMN grant_revision INTEGER NOT NULL DEFAULT 0 CHECK(grant_revision>=0);
-- Credential identity stays immutable; owner-governed role changes are policy.
DROP TRIGGER clients_identity_immutable;
CREATE TRIGGER clients_identity_immutable BEFORE UPDATE OF id,token_hash ON clients
    BEGIN SELECT RAISE(ABORT,'credential identity is immutable'); END;
DROP VIEW visible_records;
CREATE VIEW visible_records AS
    SELECT r.rowid AS record_rowid,r.namespace,r.id,r.current_revision
    FROM records r JOIN record_visibility p ON p.namespace=r.namespace AND p.record_id=r.id
    WHERE p.tombstoned=0
      AND (p.valid_from_ms IS NULL OR p.valid_from_ms<=CAST(unixepoch('subsec')*1000 AS INTEGER))
      AND (p.expires_at_ms IS NULL OR p.expires_at_ms>CAST(unixepoch('subsec')*1000 AS INTEGER))
      AND NOT EXISTS(SELECT 1 FROM relations s WHERE s.namespace=r.namespace AND s.target_id=r.id AND s.kind='supersedes');
CREATE TABLE lifecycle_receipts(
    idempotency_key TEXT PRIMARY KEY,
    request_hash BLOB NOT NULL CHECK(length(request_hash)=32),
    operation TEXT NOT NULL,
    result_json TEXT NOT NULL,
    committed_at_ms INTEGER NOT NULL
) STRICT;
CREATE TRIGGER lifecycle_receipts_no_update BEFORE UPDATE ON lifecycle_receipts
BEGIN SELECT RAISE(ABORT,'immutable lifecycle receipt'); END;
CREATE TRIGGER lifecycle_receipts_no_delete BEFORE DELETE ON lifecycle_receipts
BEGIN SELECT RAISE(ABORT,'immutable lifecycle receipt'); END;
