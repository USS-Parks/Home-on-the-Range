CREATE TABLE clients (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL CHECK(length(CAST(label AS BLOB)) BETWEEN 1 AND 128 AND instr(label,char(0))=0),
    token_hash BLOB NOT NULL UNIQUE CHECK(length(token_hash)=32),
    role TEXT NOT NULL CHECK(role IN ('reader','contributor')),
    revoked INTEGER NOT NULL DEFAULT 0 CHECK(revoked IN (0,1)),
    created_at_ms INTEGER NOT NULL CHECK(created_at_ms >= 0)
) STRICT;
CREATE TABLE client_grants (
    client_id TEXT NOT NULL REFERENCES clients(id),
    namespace TEXT NOT NULL CHECK(length(CAST(namespace AS BLOB)) BETWEEN 1 AND 128),
    PRIMARY KEY(client_id,namespace)
) STRICT;
CREATE TRIGGER clients_no_reactivate BEFORE UPDATE OF revoked ON clients WHEN OLD.revoked=1
    BEGIN SELECT RAISE(ABORT,'revocation is permanent'); END;
CREATE TRIGGER clients_identity_immutable BEFORE UPDATE OF id,token_hash,role ON clients
    BEGIN SELECT RAISE(ABORT,'credential identity is immutable'); END;
PRAGMA user_version=4;
