CREATE TABLE revision_sources (
    namespace TEXT NOT NULL,
    record_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 0 AND 15),
    reference TEXT NOT NULL CHECK(length(CAST(reference AS BLOB)) BETWEEN 1 AND 2048 AND instr(reference,char(0))=0),
    label TEXT NOT NULL CHECK(length(CAST(label AS BLOB)) <= 256 AND instr(label,char(0))=0),
    PRIMARY KEY(namespace,record_id,revision,ordinal),
    FOREIGN KEY(namespace,record_id,revision) REFERENCES revisions(namespace,record_id,revision)
) STRICT;
CREATE TABLE revision_tags (
    namespace TEXT NOT NULL,
    record_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 0 AND 31),
    tag TEXT NOT NULL CHECK(length(CAST(tag AS BLOB)) BETWEEN 1 AND 64 AND instr(tag,char(0))=0),
    PRIMARY KEY(namespace,record_id,revision,ordinal),
    UNIQUE(namespace,record_id,revision,tag),
    FOREIGN KEY(namespace,record_id,revision) REFERENCES revisions(namespace,record_id,revision)
) STRICT;
CREATE TABLE relations (
    namespace TEXT NOT NULL,
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('supports','contradicts','depends_on','supersedes','related')),
    CHECK(source_id != target_id),
    PRIMARY KEY(namespace,source_id,target_id,kind),
    FOREIGN KEY(namespace,source_id) REFERENCES records(namespace,id),
    FOREIGN KEY(namespace,target_id) REFERENCES records(namespace,id)
) STRICT;
CREATE TRIGGER sources_no_update BEFORE UPDATE ON revision_sources BEGIN SELECT RAISE(ABORT,'immutable source'); END;
CREATE TRIGGER sources_no_delete BEFORE DELETE ON revision_sources BEGIN SELECT RAISE(ABORT,'immutable source'); END;
CREATE TRIGGER tags_no_update BEFORE UPDATE ON revision_tags BEGIN SELECT RAISE(ABORT,'immutable tag'); END;
CREATE TRIGGER tags_no_delete BEFORE DELETE ON revision_tags BEGIN SELECT RAISE(ABORT,'immutable tag'); END;
PRAGMA user_version=2;
