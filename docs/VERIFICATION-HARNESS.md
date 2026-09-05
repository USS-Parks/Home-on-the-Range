# Verification harness

HOTR-03 provides `cargo xtask verify --prompt HOTR-03`. Run the native preparation command in NATIVE-BUILD.md first. The runner accepts only implemented prompt gates; an unknown prompt fails. A failed, timed-out, or skipped required command cannot produce a passing manifest.

Every run creates a new marked directory under `work/hotr-evidence/`. Files use exclusive creation and historical evidence remains intact. The runner records command outcomes, elapsed times including failures, PIDs, exit codes, binary/native hashes, source commit/dirty state, and SHA-256 of source with line endings normalized to LF. It checks source again before acceptance. Source changing during a gate fails that gate. Logs redact registered synthetic secrets and Authorization headers; a truncated trailing line is discarded. The runner does not accept real credentials on its command line or dump its environment.

Windows kernel limits apply to the dedicated runner and its ordinary descendants: 8 GiB combined committed memory, 32 active processes, and at most four logical processors' aggregate CPU time. The runner joins its own unnamed Job Object before spawning children, so child job membership is inherited at creation. The job handle stays open until runner process exit, when Windows terminates remaining descendants. An enrollment/configuration error aborts; the runner does not silently run without these controls. It only accepts a live owned Child handle for termination and refuses a mismatching PID, avoiding PID-reuse kills.

Disk usage is checked before each command and every five seconds during execution. The ceiling is 20 GiB across the project's work directory and the minimum free space is 25 GiB. This is a monitored abort threshold, not an NTFS quota or protection against a malicious executable that writes between checks. Commands are predefined trusted local build/test tools. This runner is not a sandbox for model-generated code. Logs are limited to 8 MiB per command; producer queues and retained output are bounded. Default command timeout is ten minutes; smaller negative-control and scan deadlines are explicit.

Path checks require the work subtree, reject absolute/traversal paths and existing reparse points, and check ownership markers before writing a run log. The runner has no general deletion command or arbitrary-PID kill option. Same-user malicious filesystem replacement races remain outside the product's isolation claim. Real-user vaults and existing application profiles are never test targets.

`cargo xtask fault assertion` and `cargo xtask fault timeout` intentionally exit nonzero and preserve FAIL manifests. Normal HOTR-03 verification demonstrates assertion, timeout, and log-flood rejection, redaction, unrelated-PID refusal, path refusal, and deterministic generated fixtures before product and harness build/lint/tests. A known fixed seed is checked against an independently calculated SHA-256 value. The native encryption scan remains part of the gate.

While the runner executable is active, its contract suite runs as library tests using the same contract source. This avoids asking Cargo to replace an executable that Windows has locked. The actual CLI failure paths are exercised separately with owned processes. No duplicate target directory or copied dependency tree is needed.

The Windows workflow is a minimal hosted reproduction using pinned action revisions and pinned native inputs. Its status is recorded separately from local acceptance and must be associated with the exact pushed commit. The runner does not call model providers. Subsequent prompts extend its registry and evidence rather than treating unavailable gates as passes.

## Frozen reference targets — 2026-09-05

Reference host: Windows 11 Home, 10.0.26200, AMD Ryzen 7 5800H, 8 cores / 16 logical processors, approximately 60 GiB visible RAM. Native toolchain proof: Rust 1.98.0, MSVC 19.50.35730, SQLCipher 4.18.0, SQLite 3.53.4, OpenSSL 4.0.2. The source/binary hashes in each run identify its actual inputs. Targets below are requirements, not results.

| Campaign | Frozen workload and acceptance |
|---|---|
| Prototype | 10k records, ten namespaces, 8 clients, 20 requests/s, 80/20 read/write, 15 min; write/lexical p95 <=500 ms; unexpected errors <0.1%; zero correctness/security violations |
| Scale | 100k records, 100 namespaces, 32 clients, 50 requests/s, 80/20 read/write, 30 min; write/lexical p95 <=1 s, hybrid p95 <=2 s including warm local embedding, p99 <=5 s; unexpected errors <0.1%; zero correctness/security violations |
| Soak | Four hours, 16 clients, 20 requests/s; fixed-live-corpus retained-memory growth <10% and <128 MiB; backups/restarts included |
| Races and abuse | 10k conflicting updates/retries, 100k malformed/policy-mutated requests, 100 revocation races; zero violations |
| Crash recovery | 100 tracked owned-process termination cycles; zero lost acknowledged writes; restart/unlock ready <=60 s excluding human entry; 100k restore <=5 min |
| Retrieval | 120 reviewed queries including >=40 frozen held-out queries; exact/current 100%, authorized paraphrase Recall@5 >=90%, zero prohibited results; hybrid >=10 percentage-point gain unless lexical already >=90% |
| Service bounds | Request <=256 KiB, body <=64 KiB, <=50 results, writer queue <=256, request deadline <=10 s; explicit context byte/token-estimate budgets |

The canonical PSPR contains the full definitions and remaining gates. Changing a failing target requires a dated, explicit amendment; it cannot be silently relaxed in the runner.
