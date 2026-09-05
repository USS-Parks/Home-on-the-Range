CREATE TABLE namespaces (
    name TEXT PRIMARY KEY CHECK(length(CAST(name AS BLOB)) BETWEEN 1 AND 128 AND instr(name,char(0))=0
        AND name NOT GLOB '*[^a-zA-Z0-9_./-]*'
        AND name NOT IN ('.','..') AND name NOT LIKE '/%' AND name NOT LIKE '%/'
        AND name NOT LIKE '%//%' AND ('/'||name||'/') NOT LIKE '%/./%'
        AND ('/'||name||'/') NOT LIKE '%/../%')
) STRICT;
CREATE TABLE records (
    namespace TEXT NOT NULL REFERENCES namespaces(name),
    id TEXT NOT NULL CHECK(length(CAST(id AS BLOB)) BETWEEN 1 AND 128 AND instr(id,char(0))=0
        AND id NOT GLOB '*[^a-zA-Z0-9_.-]*' AND id NOT IN ('.','..')),
    current_revision INTEGER NOT NULL CHECK(current_revision BETWEEN 1 AND 4294967295),
    PRIMARY KEY(namespace,id),
    FOREIGN KEY(namespace,id,current_revision) REFERENCES revisions(namespace,record_id,revision)
        DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE revisions (
    namespace TEXT NOT NULL,
    record_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision BETWEEN 1 AND 4294967295),
    kind TEXT NOT NULL CHECK(kind IN ('fact','preference','decision','procedure','roadmap','note')),
    body TEXT NOT NULL CHECK(length(CAST(body AS BLOB)) BETWEEN 1 AND 65536 AND instr(body,char(0))=0),
    state TEXT NOT NULL CHECK(state IN ('proposed','accepted')),
    created_at_ms INTEGER NOT NULL CHECK(created_at_ms >= 0),
    PRIMARY KEY(namespace,record_id,revision),
    FOREIGN KEY(namespace,record_id) REFERENCES records(namespace,id)
) STRICT;
CREATE TRIGGER revisions_no_update BEFORE UPDATE ON revisions BEGIN SELECT RAISE(ABORT,'immutable revision'); END;
CREATE TRIGGER revisions_no_delete BEFORE DELETE ON revisions BEGIN SELECT RAISE(ABORT,'immutable revision'); END;
CREATE TRIGGER records_identity BEFORE UPDATE OF namespace,id ON records BEGIN SELECT RAISE(ABORT,'immutable identity'); END;
CREATE TRIGGER records_sequential BEFORE UPDATE OF current_revision ON records
    WHEN NEW.current_revision != OLD.current_revision+1 BEGIN SELECT RAISE(ABORT,'nonsequential revision'); END;
PRAGMA user_version=1;
