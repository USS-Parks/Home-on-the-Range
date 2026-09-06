-- Vectors, retry state and configuration stay inside the encrypted vault.
CREATE TABLE embedding_config (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    generation INTEGER NOT NULL CHECK(generation>=0),
    port INTEGER CHECK(port BETWEEN 1 AND 65535),
    model_digest TEXT NOT NULL CHECK(length(model_digest)=64)
) STRICT;
INSERT INTO embedding_config VALUES(1,0,NULL,'0a109f422b47e3a30ba2b10eca18548e944e8a23073ee3f3e947efcf3c45e59f');
CREATE TABLE embedding_index (
    namespace TEXT NOT NULL,
    record_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    generation INTEGER NOT NULL CHECK(generation>=0),
    model_digest TEXT NOT NULL CHECK(length(model_digest)=64),
    attempts INTEGER NOT NULL CHECK(attempts BETWEEN 1 AND 3),
    due_ms INTEGER NOT NULL,
    vector BLOB CHECK(vector IS NULL OR length(vector)=3072),
    last_error TEXT,
    peer TEXT,
    completed_at_ms INTEGER,
    PRIMARY KEY(namespace,record_id),
    FOREIGN KEY(namespace,record_id,revision) REFERENCES revisions(namespace,record_id,revision)
) STRICT;
CREATE VIEW current_embeddings AS
    SELECT e.* FROM embedding_index e
    JOIN visible_records r ON r.namespace=e.namespace AND r.id=e.record_id AND r.current_revision=e.revision
    JOIN embedding_config c ON c.singleton=1 AND c.port IS NOT NULL AND c.generation=e.generation AND c.model_digest=e.model_digest
    WHERE e.vector IS NOT NULL;
