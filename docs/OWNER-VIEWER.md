# Owner viewer

HOTR-18 adds a read-only browser view to the existing Windows service. The complete local gate passed, including actual installed Chrome and credential scans. Fresh independent review and publication are tracked in the [verification ledger](VERIFICATION.md) and [evidence](evidence/HOTR-18.json).

Start and unlock a vault using the [quickstart](QUICKSTART.md), then run the built executable from the repository:

```powershell
.\work\hotr-build\target\release\hotr.exe viewer-session <vault-directory> --seconds 600
```

The authenticated owner pipe returns a plain loopback viewer URL and a separate one-time code. Open that URL, paste the code into the password field and press Open viewer. Keep the code private. It expires after 90 seconds and can be exchanged once. The resulting read-only session lasts from 5 to 600 seconds as chosen by the owner. Issuing another code ends the previous session. Each reload requires a new approval.

The viewer offers keyword search over currently visible records; current source/revision inspection; historical revisions; stale expected-revision conflict comparison and explicit relations; a metadata browser for retained records including hidden records; paginated clients with grants and revocation state; local index health; and backup status. Record bodies and source references appear as text. A semantic contradiction is not automatically detected: conflicts here mean an expected-revision mismatch or a stored relation.

Backup status covers attempts observed during the current service process. Before any attempt it says unknown. A successful receipt reports snapshot ID, completion time, size and watermark. After restart, prior backup status is unknown again. A receipt does not establish that the external snapshot still exists or that its passphrase is available; the [backup workflow](BACKUP-AND-RESTORE.md) remains the recovery authority.

## Session and browser boundary

The code field uses the explicit one-time-code autocomplete semantic. The earlier autocomplete=off field was rejected by the live gate because Chrome recorded codes in password-manager statistics. The acceptance scan checks every exchanged code and token against the retained synthetic profile, in UTF-8 and UTF-16LE, after Chrome exits. Browser behavior outside the tested version remains outside this evidence. Chromium documents the [one-time-code classification](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/components/password_manager/core/common/password_manager_constants.h).

The page holds its token only in a JavaScript closure. It never stores a credential in a URL, cookie, localStorage, sessionStorage, IndexedDB or a cache. Fetches and responses use no-store. The page clears private rendered content and form values on logout, hiding, navigation, session expiry and fatal request failure. It cancels outstanding requests and ignores stale responses. Server expiry still applies if the page closes before a logout request arrives. JavaScript garbage collection and operating-system memory are outside secure-erasure guarantees.

Every viewer API read is a POST requiring the exact bound numeric host, same Origin, same-origin Fetch Metadata, a custom header and an active viewer bearer token. The server checks the session before and after queued reads and before returning a response. Viewer tokens are separate from application credentials. The existing app API retains its rejection of browser origins. CSP forbids inline scripts, framing, form submission and external resources. Stored markup is rendered through text nodes, and source references do not become executable links.

Browser history can preserve document state independently of ordinary HTTP caching; explicit lifecycle clearing is required in addition to no-store. See [Chrome’s back/forward-cache documentation](https://developer.chrome.com/docs/web-platform/bfcache-ccns), [Fetch Metadata](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Sec-Fetch-Site), and [Request cache behavior](https://developer.mozilla.org/en-US/docs/Web/API/Request/cache).

The viewer exposes owner-wide reads, including retained historical content. Privileged changes remain on the owner CLI. The trusted Windows owner account and operating system remain the boundary; another process running as that same user is not isolated by this session. Browser extensions, screenshots and copies made by the owner are outside revocation.

## Reproduce the acceptance gate

```powershell
pwsh -NoProfile -File .cargo/verify-installed-clients.ps1 -Mode viewer
```

The local gate requires the prepared native libraries, the pinned project model, installed Chrome and the bundled Playwright runtime. It uses a new marked synthetic vault and isolated browser profile under work/hotr-tests. It runs native checks, installed-model/index/hybrid regressions, the frozen retrieval evaluation and the actual browser scenario, then scans retained synthetic storage, temporary files and evidence for plaintext canaries. HTML, CSS and JavaScript are included in the normalized source manifest. Browser acceptance and screenshots are local evidence; hosted CI alone cannot substitute for them.
