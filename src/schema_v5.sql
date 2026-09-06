CREATE TABLE record_visibility (
    namespace TEXT NOT NULL,
    record_id TEXT NOT NULL,
    tombstoned INTEGER NOT NULL DEFAULT 0 CHECK(tombstoned IN (0,1)),
    expires_at_ms INTEGER CHECK(expires_at_ms >= 0),
    PRIMARY KEY(namespace,record_id),
    FOREIGN KEY(namespace,record_id) REFERENCES records(namespace,id)
) STRICT;
INSERT INTO record_visibility(namespace,record_id) SELECT namespace,id FROM records;
CREATE TRIGGER records_visibility AFTER INSERT ON records BEGIN
    INSERT INTO record_visibility(namespace,record_id) VALUES(NEW.namespace,NEW.id);
END;
CREATE VIEW visible_records AS
    SELECT r.rowid AS record_rowid,r.namespace,r.id,r.current_revision
    FROM records r JOIN record_visibility p ON p.namespace=r.namespace AND p.record_id=r.id
    WHERE p.tombstoned=0
      AND (p.expires_at_ms IS NULL OR p.expires_at_ms > CAST(unixepoch('subsec')*1000 AS INTEGER))
      AND NOT EXISTS(SELECT 1 FROM relations s WHERE s.namespace=r.namespace AND s.target_id=r.id AND s.kind='supersedes');
CREATE INDEX relations_target_visibility ON relations(namespace,target_id,kind);
CREATE VIRTUAL TABLE record_fts USING fts5(namespace UNINDEXED, id, body, tags, sources, tokenize='unicode61');
INSERT INTO record_fts(rowid,namespace,id,body,tags,sources)
    SELECT r.rowid,r.namespace,r.id,v.body,
      coalesce((SELECT group_concat(tag,' ') FROM revision_tags t WHERE t.namespace=r.namespace AND t.record_id=r.id AND t.revision=r.current_revision),''),
      coalesce((SELECT group_concat(reference,' ') FROM revision_sources s WHERE s.namespace=r.namespace AND s.record_id=r.id AND s.revision=r.current_revision),'')
    FROM records r JOIN revisions v ON v.namespace=r.namespace AND v.record_id=r.id AND v.revision=r.current_revision;
PRAGMA user_version=5;
