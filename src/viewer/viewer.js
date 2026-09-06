(() => {
  "use strict";

  const byId = (id) => document.getElementById(id);
  const ui = {
    loginView: byId("login-view"),
    loginForm: byId("login-form"),
    loginCode: byId("login-code"),
    loginError: byId("login-error"),
    viewerView: byId("viewer-view"),
    logoutButton: byId("logout-button"),
    sessionStatus: byId("session-status"),
    workspaceStatus: byId("workspace-status"),
    namespaceOptions: byId("namespace-options"),
    searchForm: byId("search-form"),
    searchNamespace: byId("search-namespace"),
    searchQuery: byId("search-query"),
    listButton: byId("list-button"),
    searchSummary: byId("search-summary"),
    searchResults: byId("search-results"),
    searchPager: byId("search-pager"),
    searchPrevious: byId("search-previous"),
    searchNext: byId("search-next"),
    searchPageLabel: byId("search-page-label"),
    retainedForm: byId("retained-form"),
    retainedNamespace: byId("retained-namespace"),
    retainedSummary: byId("retained-summary"),
    retainedResults: byId("retained-results"),
    retainedPager: byId("retained-pager"),
    retainedPrevious: byId("retained-previous"),
    retainedNext: byId("retained-next"),
    retainedPageLabel: byId("retained-page-label"),
    inspectForm: byId("inspect-form"),
    inspectNamespace: byId("inspect-namespace"),
    inspectId: byId("inspect-id"),
    expectedRevision: byId("expected-revision"),
    inspectResults: byId("inspect-results"),
    historySection: byId("history-section"),
    historySummary: byId("history-summary"),
    historyResults: byId("history-results"),
    historyPager: byId("history-pager"),
    historyPrevious: byId("history-previous"),
    historyNext: byId("history-next"),
    historyPageLabel: byId("history-page-label"),
    clientsRefresh: byId("clients-refresh"),
    clientsSummary: byId("clients-summary"),
    clientsResults: byId("clients-results"),
    clientsPager: byId("clients-pager"),
    clientsPrevious: byId("clients-previous"),
    clientsNext: byId("clients-next"),
    clientsPageLabel: byId("clients-page-label"),
    indexRefresh: byId("index-refresh"),
    indexResults: byId("index-results"),
    backupRefresh: byId("backup-refresh"),
    backupResults: byId("backup-results"),
  };

  const navButtons = Array.from(document.querySelectorAll(".nav-button"));
  const panels = Array.from(document.querySelectorAll("[data-panel]"));
  const activeRequests = new Set();
  const SEARCH_LIMIT = 10;
  const HISTORY_LIMIT = 5;
  const PAGE_BYTE_BUDGET = 262144;
  const PAGE_TOKEN_BUDGET = 262144;

  let token = null;
  let sessionEpoch = 0;
  let viewEpoch = 0;
  let sessionDeadline = 0;
  let expiryTimer = null;
  let pingTimer = null;
  let currentView = "search";
  let searchState = blankSearchState();
  let retainedState = blankOffsetState();
  let historyState = blankHistoryState();
  let clientsState = blankOffsetState();

  class StaleRequestError extends Error {}
  class InputResponseError extends Error {
    constructor(code) {
      super(code);
      this.code = code;
    }
  }
  class SessionResponseError extends Error {}

  function blankSearchState() {
    return { mode: null, namespace: "", query: "", offset: 0, nextOffset: null, total: 0 };
  }

  function blankOffsetState() {
    return { namespace: "", offsets: [0], position: 0, nextOffset: null, total: 0 };
  }

  function blankHistoryState() {
    return { namespace: "", id: "", offset: 0, nextOffset: null, total: 0 };
  }

  function text(value, fallback = "—") {
    if (value === null || value === undefined || value === "") {
      return fallback;
    }
    return String(value);
  }

  function numberText(value) {
    const number = Number(value);
    return Number.isFinite(number) ? new Intl.NumberFormat().format(number) : "—";
  }

  function dateText(value) {
    if (value === null || value === undefined || value === "") {
      return "—";
    }
    const number = Number(value);
    if (!Number.isFinite(number)) {
      return "—";
    }
    const date = new Date(number);
    return Number.isNaN(date.getTime()) ? "—" : date.toLocaleString();
  }

  function array(value) {
    return Array.isArray(value) ? value : [];
  }

  function jsonText(value) {
    try {
      return JSON.stringify(value, null, 2);
    } catch (_error) {
      return "Watermark could not be displayed.";
    }
  }

  function element(tag, className, value) {
    const node = document.createElement(tag);
    if (className) {
      node.className = className;
    }
    if (value !== undefined) {
      node.textContent = text(value, "");
    }
    return node;
  }

  function badge(label, kind = "neutral") {
    return element("span", `badge badge-${kind}`, label);
  }

  function clearNode(node) {
    node.replaceChildren();
  }

  function setWorkspaceStatus(message = "", tone = "") {
    ui.workspaceStatus.textContent = message;
    if (tone) {
      ui.workspaceStatus.dataset.tone = tone;
    } else {
      delete ui.workspaceStatus.dataset.tone;
    }
  }

  function showState(container, title, detail, isError = false) {
    clearNode(container);
    const box = element("div", isError ? "error-state" : "empty-state");
    box.append(element("h3", "", title));
    if (detail) {
      box.append(element("p", "", detail));
    }
    container.append(box);
  }

  function detailItem(label, value, monospace = false) {
    const item = element("div", "detail-item");
    item.append(element("span", "detail-label", label));
    item.append(element("span", monospace ? "detail-value mono" : "detail-value", value));
    return item;
  }

  function appendTags(container, tags) {
    const values = array(tags);
    if (!values.length) {
      return;
    }
    const list = element("div", "tag-list");
    values.forEach((tag) => list.append(element("span", "tag", tag)));
    container.append(list);
  }

  function appendSources(container, sources) {
    const values = array(sources);
    const label = element("p", "section-label", "Source references");
    container.append(label);
    if (!values.length) {
      container.append(element("p", "meta-text", "No source references recorded."));
      return;
    }
    const list = element("ul", "plain-list");
    values.forEach((source) => {
      const item = element("li");
      item.append(element("span", "", text(source && source.label, "Unlabeled source")));
      item.append(element("span", "source-reference", text(source && source.reference, "No reference")));
      list.append(item);
    });
    container.append(list);
  }

  function renderRevision(record, options = {}) {
    const visible = options.visible;
    const card = element("article", "result-card");
    if (visible === true) {
      card.dataset.visibility = "visible";
    } else if (visible === false) {
      card.dataset.visibility = "hidden";
    }

    const top = element("div", "meta-line");
    const title = element("h3", "result-title mono", text(record && record.id, "Unnamed record"));
    top.append(title);
    top.append(badge(`r${text(record && record.revision, "?")}`, "neutral"));
    top.append(badge(text(record && record.state, "unknown state"), "neutral"));
    if (visible === true) {
      top.append(badge("Current visible", "visible"));
    } else if (visible === false) {
      top.append(badge("Retained / hidden", "hidden"));
    }
    card.append(top);

    const meta = element("div", "detail-grid");
    meta.append(detailItem("Namespace", text(record && record.namespace), true));
    meta.append(detailItem("Kind", text(record && record.kind)));
    meta.append(detailItem("Created", dateText(record && record.created_at_ms)));
    meta.append(detailItem("Revision", text(record && record.revision)));
    card.append(meta);

    if (options.showBody !== false) {
      const body = element("pre", "record-body");
      body.textContent = text(record && record.body, "No body text.");
      body.tabIndex = 0;
      body.setAttribute("aria-label", `Record body for ${text(record && record.id, "record")}`);
      card.append(body);
      appendTags(card, record && record.tags);
      appendSources(card, record && record.sources);
    }
    return card;
  }

  function operationCode(payload, response) {
    const code = payload && payload.error && payload.error.code;
    return text(code, `HTTP_${response.status}`);
  }

  async function parseResponse(response) {
    try {
      return await response.json();
    } catch (_error) {
      return null;
    }
  }

  function registerRequest(controller, viewScoped) {
    const entry = { controller, viewScoped };
    activeRequests.add(entry);
    return entry;
  }

  function abortRequests(onlyViewScoped) {
    for (const entry of activeRequests) {
      if (!onlyViewScoped || entry.viewScoped) {
        entry.controller.abort();
        activeRequests.delete(entry);
      }
    }
  }

  function requestIsStale(capturedSession, capturedView, viewScoped) {
    return capturedSession !== sessionEpoch || !token || (viewScoped && capturedView !== viewEpoch);
  }

  async function read(body, options = {}) {
    const viewScoped = options.viewScoped !== false;
    const capturedSession = sessionEpoch;
    const capturedView = viewEpoch;
    if (!token) {
      throw new StaleRequestError();
    }

    const controller = new AbortController();
    const entry = registerRequest(controller, viewScoped);
    let response;
    let payload;
    try {
      response = await fetch("/viewer/api/read", {
        method: "POST",
        headers: {
          Authorization: `Bearer ${token}`,
          "Content-Type": "application/json",
          "X-HOTR-Viewer": "1",
        },
        body: JSON.stringify(body),
        cache: "no-store",
        credentials: "omit",
        signal: controller.signal,
      });
      payload = await parseResponse(response);
    } catch (error) {
      if (controller.signal.aborted || requestIsStale(capturedSession, capturedView, viewScoped)) {
        throw new StaleRequestError();
      }
      endSession("Viewer locked or unavailable. Generate a new one-time session and try again.", false);
      throw new SessionResponseError();
    } finally {
      activeRequests.delete(entry);
    }

    if (requestIsStale(capturedSession, capturedView, viewScoped)) {
      throw new StaleRequestError();
    }
    if (!response.ok) {
      const code = operationCode(payload, response);
      if (response.status === 400 || (response.status === 404 && code === "not_found")) {
        throw new InputResponseError(code);
      }
      const serverCanCloseSession = response.status !== 401 && response.status !== 403 && response.status !== 503;
      endSession(`Viewer locked or unavailable (${code}). Generate a new one-time session and try again.`, serverCanCloseSession);
      throw new SessionResponseError();
    }
    if (!payload || typeof payload !== "object") {
      endSession("Viewer locked or unavailable. The local service returned an invalid response.", true);
      throw new SessionResponseError();
    }
    return payload;
  }

  function handleOperationError(error, container) {
    if (error instanceof StaleRequestError || error instanceof SessionResponseError) {
      return;
    }
    const code = error instanceof InputResponseError ? error.code : "UNEXPECTED_RESPONSE";
    showState(container, "Could not load this view", `The service rejected the input (${code}). Check the fields and try again.`, true);
    setWorkspaceStatus(`Input rejected: ${code}`, "error");
  }

  function bestEffortLogout() {
    if (!token) {
      return;
    }
    fetch("/viewer/api/logout", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
        "X-HOTR-Viewer": "1",
      },
      body: "{}",
      cache: "no-store",
      credentials: "omit",
      keepalive: true,
    }).catch(() => {});
  }

  function clearTimers() {
    if (expiryTimer !== null) {
      clearTimeout(expiryTimer);
      expiryTimer = null;
    }
    if (pingTimer !== null) {
      clearTimeout(pingTimer);
      pingTimer = null;
    }
  }

  function clearPrivateData() {
    ui.searchForm.reset();
    ui.retainedForm.reset();
    ui.inspectForm.reset();
    ui.loginCode.value = "";
    [
      ui.namespaceOptions,
      ui.searchSummary,
      ui.searchResults,
      ui.retainedSummary,
      ui.retainedResults,
      ui.inspectResults,
      ui.historySummary,
      ui.historyResults,
      ui.clientsSummary,
      ui.clientsResults,
      ui.indexResults,
      ui.backupResults,
      ui.searchPageLabel,
      ui.retainedPageLabel,
      ui.historyPageLabel,
      ui.clientsPageLabel,
    ].forEach(clearNode);
    ui.searchPager.hidden = true;
    ui.retainedPager.hidden = true;
    ui.historySection.hidden = true;
    ui.historyPager.hidden = true;
    ui.clientsPager.hidden = true;
    setWorkspaceStatus();
    searchState = blankSearchState();
    retainedState = blankOffsetState();
    historyState = blankHistoryState();
    clientsState = blankOffsetState();
  }

  function presentLogin(message = "") {
    ui.viewerView.hidden = true;
    ui.loginView.hidden = false;
    ui.logoutButton.hidden = true;
    ui.sessionStatus.textContent = "No active viewer session";
    ui.loginError.textContent = message;
    ui.loginError.hidden = !message;
  }

  function endSession(message, notifyServer) {
    if (notifyServer) {
      bestEffortLogout();
    }
    sessionEpoch += 1;
    viewEpoch += 1;
    abortRequests(false);
    clearTimers();
    token = null;
    sessionDeadline = 0;
    clearPrivateData();
    currentView = "search";
    panels.forEach((panel) => {
      panel.hidden = panel.dataset.panel !== "search";
    });
    navButtons.forEach((button) => {
      if (button.dataset.view === "search") {
        button.setAttribute("aria-current", "page");
      } else {
        button.removeAttribute("aria-current");
      }
    });
    presentLogin(message);
  }

  function scheduleExpiry() {
    if (!token) {
      return;
    }
    const remaining = Math.max(0, sessionDeadline - performance.now());
    expiryTimer = setTimeout(() => {
      endSession("Viewer session expired. Generate a new one-time session to continue.", false);
    }, remaining);
  }

  function updateSessionCountdown() {
    if (!token) {
      return;
    }
    const remaining = Math.max(0, Math.ceil((sessionDeadline - performance.now()) / 1000));
    ui.sessionStatus.textContent = `Read-only session · ${remaining}s remaining`;
  }

  function schedulePing() {
    if (!token) {
      return;
    }
    pingTimer = setTimeout(async () => {
      if (!token) {
        return;
      }
      if (performance.now() >= sessionDeadline) {
        endSession("Viewer session expired. Generate a new one-time session to continue.", false);
        return;
      }
      try {
        await read({ operation: "ping" }, { viewScoped: false });
        updateSessionCountdown();
      } catch (error) {
        if (!(error instanceof StaleRequestError) && !(error instanceof SessionResponseError)) {
          endSession("Viewer locked or unavailable. Generate a new one-time session and try again.", false);
        }
      }
      schedulePing();
    }, 5000);
  }

  function makePage(namespace, limit, offset) {
    return {
      namespace,
      limit,
      offset,
      byte_budget: PAGE_BYTE_BUDGET,
      token_budget: PAGE_TOKEN_BUDGET,
    };
  }

  async function loadNamespaces() {
    try {
      const data = await read({ operation: "namespaces", offset: 0 }, { viewScoped: false });
      clearNode(ui.namespaceOptions);
      array(data.namespaces).slice(0, 50).forEach((namespace) => {
        const option = document.createElement("option");
        option.value = text(namespace, "");
        ui.namespaceOptions.append(option);
      });
    } catch (error) {
      if (!(error instanceof StaleRequestError) && !(error instanceof SessionResponseError)) {
        endSession("Viewer locked or unavailable. Namespace discovery failed.", false);
      }
    }
  }

  function switchView(name, autoLoad = true) {
    if (!token || !panels.some((panel) => panel.dataset.panel === name)) {
      return;
    }
    abortRequests(true);
    viewEpoch += 1;
    currentView = name;
    panels.forEach((panel) => {
      panel.hidden = panel.dataset.panel !== name;
    });
    navButtons.forEach((button) => {
      if (button.dataset.view === name) {
        button.setAttribute("aria-current", "page");
      } else {
        button.removeAttribute("aria-current");
      }
    });
    setWorkspaceStatus();
    if (autoLoad && name === "clients") {
      clientsState = blankOffsetState();
      void loadClients(0, true);
    } else if (autoLoad && name === "index") {
      void loadIndex();
    } else if (autoLoad && name === "backup") {
      void loadBackup();
    }
  }

  function beginViewOperation() {
    abortRequests(true);
    viewEpoch += 1;
  }

  function addInspectButton(container, namespace, id) {
    const button = element("button", "button button-quiet", "Inspect record");
    button.type = "button";
    button.addEventListener("click", () => {
      switchView("record", false);
      ui.inspectNamespace.value = text(namespace, "");
      ui.inspectId.value = text(id, "");
      ui.expectedRevision.value = "";
      void inspectRecord();
    });
    container.append(button);
  }

  function renderSearchPage(data) {
    clearNode(ui.searchResults);
    const records = array(data.records);
    if (!records.length) {
      showState(ui.searchResults, "No current visible records", searchState.mode === "search" ? "No visible records matched those literal keywords." : "This namespace has no current visible records.");
    } else {
      records.forEach((record) => {
        const card = renderRevision(record, { visible: true });
        const actions = element("div", "button-row");
        addInspectButton(actions, record.namespace, record.id);
        card.append(actions);
        ui.searchResults.append(card);
      });
    }
    searchState.total = Number(data.total) || 0;
    searchState.nextOffset = Number.isInteger(data.next_offset) ? data.next_offset : null;
    const omitted = Number(data.omitted_for_budget) || 0;
    const modeLabel = searchState.mode === "search" ? "Keyword results" : "Current visible records";
    ui.searchSummary.textContent = `${modeLabel}: ${records.length} shown of ${numberText(searchState.total)}${omitted ? ` · ${numberText(omitted)} omitted for response budget` : ""}`;
    updateSearchPager();
  }

  function updateSearchPager() {
    const hasPrevious = searchState.offset > 0;
    const hasNext = searchState.nextOffset !== null;
    ui.searchPager.hidden = !hasPrevious && !hasNext;
    ui.searchPrevious.disabled = !hasPrevious;
    ui.searchNext.disabled = !hasNext;
    ui.searchPageLabel.textContent = searchState.total ? `${numberText(searchState.offset + 1)}–${numberText(Math.min(searchState.offset + SEARCH_LIMIT, searchState.total))} of ${numberText(searchState.total)}` : "No results";
  }

  async function runSearch(mode, offset = 0, preserveInput = false) {
    beginViewOperation();
    if (!preserveInput) {
      searchState.namespace = ui.searchNamespace.value.trim();
      searchState.query = ui.searchQuery.value.trim();
      searchState.mode = mode;
    }
    searchState.offset = offset;
    clearNode(ui.searchSummary);
    clearNode(ui.searchResults);
    ui.searchPager.hidden = true;

    if (!searchState.namespace) {
      showState(ui.searchResults, "Namespace required", "Enter a namespace before searching or browsing.", true);
      setWorkspaceStatus("Enter a namespace.", "error");
      return;
    }
    if (mode === "search" && !searchState.query) {
      showState(ui.searchResults, "Keywords required", "Enter one or more literal keywords, or use Browse current.", true);
      setWorkspaceStatus("Enter search keywords.", "error");
      return;
    }

    setWorkspaceStatus(mode === "search" ? "Searching current visible records…" : "Loading current visible records…");
    const page = makePage(searchState.namespace, SEARCH_LIMIT, offset);
    const body = mode === "search"
      ? { operation: "search", query: { page, query: searchState.query } }
      : { operation: "list", page };
    try {
      const data = await read(body);
      renderSearchPage(data);
      setWorkspaceStatus("Results loaded.", "success");
    } catch (error) {
      handleOperationError(error, ui.searchResults);
    }
  }

  function updateOffsetPager(state, pager, previous, next, label) {
    const hasPrevious = state.position > 0;
    const hasNext = state.nextOffset !== null;
    pager.hidden = !hasPrevious && !hasNext;
    previous.disabled = !hasPrevious;
    next.disabled = !hasNext;
    const offset = state.offsets[state.position] || 0;
    label.textContent = state.total ? `Starting at ${numberText(offset + 1)} of ${numberText(state.total)}` : "No results";
  }

  function renderRetainedPage(data) {
    clearNode(ui.retainedResults);
    const records = array(data.records);
    if (!records.length) {
      showState(ui.retainedResults, "No retained records", "No retained record metadata was returned for this namespace.");
    } else {
      records.forEach((record) => {
        const card = renderRevision(record, { visible: record.visible === true, showBody: false });
        const actions = element("div", "button-row");
        addInspectButton(actions, record.namespace, record.id);
        card.append(actions);
        ui.retainedResults.append(card);
      });
    }
    retainedState.total = Number(data.total) || 0;
    retainedState.nextOffset = Number.isInteger(data.next_offset) ? data.next_offset : null;
    ui.retainedSummary.textContent = `Retained metadata: ${records.length} shown of ${numberText(retainedState.total)}. Hidden entries are not current visible search results.`;
    updateOffsetPager(retainedState, ui.retainedPager, ui.retainedPrevious, ui.retainedNext, ui.retainedPageLabel);
  }

  async function loadRetained(offset, reset) {
    beginViewOperation();
    if (reset) {
      retainedState = blankOffsetState();
      retainedState.namespace = ui.retainedNamespace.value.trim();
    }
    clearNode(ui.retainedSummary);
    clearNode(ui.retainedResults);
    ui.retainedPager.hidden = true;
    if (!retainedState.namespace) {
      showState(ui.retainedResults, "Namespace required", "Enter a namespace to browse retained record IDs.", true);
      setWorkspaceStatus("Enter a namespace.", "error");
      return;
    }
    setWorkspaceStatus("Loading retained record metadata…");
    try {
      const data = await read({ operation: "records", namespace: retainedState.namespace, offset });
      renderRetainedPage(data);
      setWorkspaceStatus("Retained metadata loaded.", "success");
    } catch (error) {
      handleOperationError(error, ui.retainedResults);
    }
  }

  function renderRelations(container, relations) {
    container.append(element("p", "section-label", "Relations"));
    const values = array(relations);
    if (!values.length) {
      container.append(element("p", "meta-text", "No relations recorded."));
      return;
    }
    const list = element("ul", "plain-list");
    values.forEach((relation) => {
      list.append(element("li", "mono", `${text(relation && relation.source_id)} → ${text(relation && relation.target_id)} · ${text(relation && relation.kind, "relation")}`));
    });
    container.append(list);
  }

  function renderInspection(data) {
    clearNode(ui.inspectResults);
    if (data.conflict === true) {
      ui.inspectResults.append(element("div", "conflict-banner", `Conflict: expected revision ${text(data.expected_revision)}, current revision ${text(data.current && data.current.revision)}.`));
    }
    const wrapper = element("section", "health-card");
    const header = element("div", "meta-line");
    header.append(element("h2", "result-title mono", text(data.current && data.current.id, "Record")));
    header.append(data.visible === true ? badge("Current visible", "visible") : badge("Retained / hidden", "hidden"));
    if (data.policy && data.policy.tombstoned === true) {
      header.append(badge("Tombstoned", "bad"));
    }
    wrapper.append(header);

    const policy = data.policy || {};
    const policyGrid = element("div", "detail-grid");
    policyGrid.append(detailItem("Visibility", data.visible === true ? "Current visible" : "Retained / hidden"));
    policyGrid.append(detailItem("Tombstoned", policy.tombstoned === true ? "Yes" : "No"));
    policyGrid.append(detailItem("Valid from", dateText(policy.valid_from_ms)));
    policyGrid.append(detailItem("Expires", policy.expires_at_ms === null ? "No expiry" : dateText(policy.expires_at_ms)));
    wrapper.append(policyGrid);

    wrapper.append(renderRevision(data.current, { visible: data.visible === true }));
    renderRelations(wrapper, data.relations);
    ui.inspectResults.append(wrapper);
  }

  async function inspectRecord() {
    beginViewOperation();
    const namespace = ui.inspectNamespace.value.trim();
    const id = ui.inspectId.value.trim();
    const expectedText = ui.expectedRevision.value.trim();
    let expectedRevision = null;
    clearNode(ui.inspectResults);
    clearNode(ui.historyResults);
    clearNode(ui.historySummary);
    ui.historySection.hidden = true;
    ui.historyPager.hidden = true;
    if (!namespace || !id) {
      showState(ui.inspectResults, "Record coordinates required", "Enter both a namespace and record ID.", true);
      setWorkspaceStatus("Enter a namespace and record ID.", "error");
      return;
    }
    if (expectedText) {
      expectedRevision = Number(expectedText);
      if (!Number.isSafeInteger(expectedRevision) || expectedRevision < 1) {
        showState(ui.inspectResults, "Invalid expected revision", "Expected revision must be a positive whole number.", true);
        setWorkspaceStatus("Correct the expected revision.", "error");
        return;
      }
    }
    setWorkspaceStatus("Inspecting record…");
    try {
      const data = await read({ operation: "inspect", query: { namespace, id, expected_revision: expectedRevision } });
      renderInspection(data);
      historyState = { namespace, id, offset: 0, nextOffset: null, total: 0 };
      ui.historySection.hidden = false;
      setWorkspaceStatus("Record loaded. Loading revision history…", "success");
      await loadHistory(0);
    } catch (error) {
      handleOperationError(error, ui.inspectResults);
    }
  }

  function renderHistoryPage(data) {
    clearNode(ui.historyResults);
    const records = array(data.records);
    if (!records.length) {
      showState(ui.historyResults, "No revision history", "No retained revisions were returned for this record.");
    } else {
      records.forEach((record) => ui.historyResults.append(renderRevision(record)));
    }
    historyState.total = Number(data.total) || 0;
    historyState.nextOffset = Number.isInteger(data.next_offset) ? data.next_offset : null;
    const omitted = Number(data.omitted_for_budget) || 0;
    ui.historySummary.textContent = `${records.length} shown of ${numberText(historyState.total)}${omitted ? ` · ${numberText(omitted)} omitted for response budget` : ""}`;
    const hasPrevious = historyState.offset > 0;
    const hasNext = historyState.nextOffset !== null;
    ui.historyPager.hidden = !hasPrevious && !hasNext;
    ui.historyPrevious.disabled = !hasPrevious;
    ui.historyNext.disabled = !hasNext;
    ui.historyPageLabel.textContent = historyState.total ? `${numberText(historyState.offset + 1)}–${numberText(Math.min(historyState.offset + HISTORY_LIMIT, historyState.total))} of ${numberText(historyState.total)}` : "No revisions";
  }

  async function loadHistory(offset) {
    beginViewOperation();
    historyState.offset = offset;
    clearNode(ui.historyResults);
    clearNode(ui.historySummary);
    ui.historyPager.hidden = true;
    try {
      const page = makePage(historyState.namespace, HISTORY_LIMIT, offset);
      const data = await read({ operation: "history", query: { page, id: historyState.id } });
      renderHistoryPage(data);
      setWorkspaceStatus("Record and history loaded.", "success");
    } catch (error) {
      handleOperationError(error, ui.historyResults);
    }
  }

  function renderClientsPage(data) {
    clearNode(ui.clientsResults);
    const clients = array(data.clients);
    if (!clients.length) {
      showState(ui.clientsResults, "No clients", "No client grants were returned.");
    } else {
      clients.forEach((client) => {
        const card = element("article", "result-card client-card");
        const top = element("div", "meta-line");
        top.append(element("h3", "result-title", text(client.label, "Unlabeled client")));
        top.append(client.revoked === true ? badge("Revoked", "bad") : badge("Active", "good"));
        top.append(badge(text(client.role, "unknown role"), "neutral"));
        card.append(top);
        const details = element("div", "detail-grid");
        details.append(detailItem("Client ID", text(client.client_id), true));
        details.append(detailItem("Grant revision", text(client.grant_revision)));
        card.append(details);
        card.append(element("p", "section-label", "Granted namespaces"));
        if (array(client.namespaces).length) {
          appendTags(card, client.namespaces);
        } else {
          card.append(element("p", "meta-text", "No namespaces granted."));
        }
        ui.clientsResults.append(card);
      });
    }
    clientsState.total = Number(data.total) || 0;
    clientsState.nextOffset = Number.isInteger(data.next_offset) ? data.next_offset : null;
    ui.clientsSummary.textContent = `${clients.length} clients shown of ${numberText(clientsState.total)}`;
    updateOffsetPager(clientsState, ui.clientsPager, ui.clientsPrevious, ui.clientsNext, ui.clientsPageLabel);
  }

  async function loadClients(offset, reset) {
    beginViewOperation();
    if (reset) {
      clientsState = blankOffsetState();
    }
    clearNode(ui.clientsSummary);
    clearNode(ui.clientsResults);
    ui.clientsPager.hidden = true;
    setWorkspaceStatus("Loading clients and grants…");
    try {
      const data = await read({ operation: "clients", offset });
      renderClientsPage(data);
      setWorkspaceStatus("Clients and grants loaded.", "success");
    } catch (error) {
      handleOperationError(error, ui.clientsResults);
    }
  }

  function renderIndex(data) {
    clearNode(ui.indexResults);
    const card = element("section", "health-card");
    const top = element("div", "meta-line");
    top.append(element("h2", "", "Index service"));
    top.append(data.enabled === true ? badge("Enabled", "good") : badge("Disabled", "warning"));
    if ((Number(data.failed) || 0) > 0) {
      top.append(badge(`${numberText(data.failed)} failed`, "bad"));
    }
    card.append(top);
    const grid = element("div", "health-grid");
    [
      ["Generation", data.generation],
      ["Port", data.port],
      ["Dimensions", data.dimensions],
      ["Visible records", data.visible],
      ["Indexed", data.indexed],
      ["Pending", data.pending],
      ["Failed", data.failed],
      ["Max attempts", data.max_attempts],
      ["Model", data.model],
      ["Model digest", data.model_digest],
      ["Last peer", data.last_peer],
    ].forEach(([label, value]) => grid.append(detailItem(label, text(value), label === "Model digest" || label === "Last peer")));
    card.append(grid);
    card.append(element("p", "section-label", "Last error"));
    const lastError = element("pre", "record-body", text(data.last_error, "No error reported."));
    lastError.tabIndex = 0;
    lastError.setAttribute("aria-label", "Latest index error details");
    card.append(lastError);
    ui.indexResults.append(card);
  }

  async function loadIndex() {
    beginViewOperation();
    clearNode(ui.indexResults);
    setWorkspaceStatus("Loading index health…");
    try {
      const data = await read({ operation: "index" });
      renderIndex(data);
      setWorkspaceStatus("Index health loaded.", "success");
    } catch (error) {
      handleOperationError(error, ui.indexResults);
    }
  }

  function renderBackup(data) {
    clearNode(ui.backupResults);
    const status = text(data.status, "unknown");
    const card = element("section", "health-card");
    const top = element("div", "meta-line");
    top.append(element("h2", "", "Backup observation"));
    top.append(status === "succeeded" ? badge("Succeeded", "good") : status === "failed" ? badge("Failed", "bad") : badge("Unknown", "warning"));
    card.append(top);
    const grid = element("div", "detail-grid");
    grid.append(detailItem("Scope", text(data.scope, "current_service_process")));
    grid.append(detailItem("Last attempt", data.last_attempt_at_ms === null ? "No attempt observed" : dateText(data.last_attempt_at_ms)));
    card.append(grid);
    if (data.last_success) {
      card.append(element("p", "section-label", "Last successful receipt"));
      const success = element("div", "health-grid");
      success.append(detailItem("Snapshot ID", text(data.last_success.snapshot_id), true));
      success.append(detailItem("Completed", dateText(data.last_success.completed_at_ms)));
      success.append(detailItem("Bytes", numberText(data.last_success.bytes)));
      if (data.last_success.watermark === null || typeof data.last_success.watermark !== "object") {
        success.append(detailItem("Watermark", text(data.last_success.watermark), true));
      }
      card.append(success);
      if (data.last_success.watermark && typeof data.last_success.watermark === "object") {
        card.append(element("p", "section-label", "Watermark"));
        const watermark = element("pre", "record-body", jsonText(data.last_success.watermark));
        watermark.tabIndex = 0;
        watermark.setAttribute("aria-label", "Backup watermark details");
        card.append(watermark);
      }
      card.append(element("p", "meta-text", "A successful historical receipt reports what the service recorded; it does not verify that backup files remain on disk."));
    } else {
      card.append(element("p", "meta-text", "No successful backup receipt is known to this service process."));
    }
    ui.backupResults.append(card);
  }

  async function loadBackup() {
    beginViewOperation();
    clearNode(ui.backupResults);
    setWorkspaceStatus("Loading backup status…");
    try {
      const data = await read({ operation: "backup" });
      renderBackup(data);
      setWorkspaceStatus("Backup status loaded.", "success");
    } catch (error) {
      handleOperationError(error, ui.backupResults);
    }
  }

  async function login(event) {
    event.preventDefault();
    const code = ui.loginCode.value.trim();
    ui.loginCode.value = "";
    ui.loginError.hidden = true;
    ui.loginError.textContent = "";
    if (!/^[0-9a-fA-F]{64}$/.test(code)) {
      ui.loginError.textContent = "Enter the complete 64-character hexadecimal one-time code.";
      ui.loginError.hidden = false;
      ui.loginCode.focus();
      return;
    }

    abortRequests(false);
    clearTimers();
    clearPrivateData();
    const capturedSession = ++sessionEpoch;
    const loginStartedAt = performance.now();
    const controller = new AbortController();
    const entry = registerRequest(controller, false);
    let response;
    let payload;
    try {
      response = await fetch("/viewer/api/session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-HOTR-Viewer": "1",
        },
        body: JSON.stringify({ code }),
        cache: "no-store",
        credentials: "omit",
        signal: controller.signal,
      });
      payload = await parseResponse(response);
    } catch (_error) {
      if (capturedSession === sessionEpoch && !controller.signal.aborted) {
        presentLogin("Viewer locked or unavailable. Confirm the local service is unlocked and generate a new one-time code.");
      }
      return;
    } finally {
      activeRequests.delete(entry);
    }

    if (capturedSession !== sessionEpoch) {
      return;
    }
    if (!response.ok) {
      const codeName = operationCode(payload, response);
      const message = response.status === 400
        ? `The one-time code was rejected (${codeName}). Generate a fresh code and try again.`
        : `Viewer locked or unavailable (${codeName}). Unlock the local service, then generate a new code.`;
      presentLogin(message);
      return;
    }
    if (!payload || !/^[0-9a-fA-F]{64}$/.test(text(payload.token, "")) || !Number.isFinite(Number(payload.expires_in_seconds)) || Number(payload.expires_in_seconds) <= 0) {
      presentLogin("Viewer locked or unavailable. The local service returned an invalid session response.");
      return;
    }

    const responseDeadline = loginStartedAt + (Number(payload.expires_in_seconds) * 1000);
    if (responseDeadline <= performance.now()) {
      presentLogin("The viewer session expired before it could open. Generate a new one-time code and try again.");
      return;
    }
    token = payload.token;
    sessionDeadline = responseDeadline;
    ui.loginView.hidden = true;
    ui.viewerView.hidden = false;
    ui.logoutButton.hidden = false;
    currentView = "search";
    switchView("search", false);
    updateSessionCountdown();
    scheduleExpiry();
    schedulePing();
    setWorkspaceStatus("Read-only viewer session opened.", "success");
    void loadNamespaces();
  }

  ui.loginForm.addEventListener("submit", login);
  ui.logoutButton.addEventListener("click", () => {
    endSession("Viewer session ended. Generate a new one-time session to return.", true);
    ui.loginCode.focus();
  });
  navButtons.forEach((button) => button.addEventListener("click", () => switchView(button.dataset.view)));
  ui.searchForm.addEventListener("submit", (event) => {
    event.preventDefault();
    void runSearch("search");
  });
  ui.listButton.addEventListener("click", () => void runSearch("list"));
  ui.searchPrevious.addEventListener("click", () => void runSearch(searchState.mode, Math.max(0, searchState.offset - SEARCH_LIMIT), true));
  ui.searchNext.addEventListener("click", () => {
    if (searchState.nextOffset !== null) {
      void runSearch(searchState.mode, searchState.nextOffset, true);
    }
  });
  ui.retainedForm.addEventListener("submit", (event) => {
    event.preventDefault();
    void loadRetained(0, true);
  });
  ui.retainedPrevious.addEventListener("click", () => {
    if (retainedState.position > 0) {
      retainedState.position -= 1;
      void loadRetained(retainedState.offsets[retainedState.position], false);
    }
  });
  ui.retainedNext.addEventListener("click", () => {
    if (retainedState.nextOffset !== null) {
      retainedState.position += 1;
      retainedState.offsets = retainedState.offsets.slice(0, retainedState.position);
      retainedState.offsets.push(retainedState.nextOffset);
      void loadRetained(retainedState.nextOffset, false);
    }
  });
  ui.inspectForm.addEventListener("submit", (event) => {
    event.preventDefault();
    void inspectRecord();
  });
  ui.historyPrevious.addEventListener("click", () => void loadHistory(Math.max(0, historyState.offset - HISTORY_LIMIT)));
  ui.historyNext.addEventListener("click", () => {
    if (historyState.nextOffset !== null) {
      void loadHistory(historyState.nextOffset);
    }
  });
  ui.clientsRefresh.addEventListener("click", () => void loadClients(0, true));
  ui.clientsPrevious.addEventListener("click", () => {
    if (clientsState.position > 0) {
      clientsState.position -= 1;
      void loadClients(clientsState.offsets[clientsState.position], false);
    }
  });
  ui.clientsNext.addEventListener("click", () => {
    if (clientsState.nextOffset !== null) {
      clientsState.position += 1;
      clientsState.offsets = clientsState.offsets.slice(0, clientsState.position);
      clientsState.offsets.push(clientsState.nextOffset);
      void loadClients(clientsState.nextOffset, false);
    }
  });
  ui.indexRefresh.addEventListener("click", () => void loadIndex());
  ui.backupRefresh.addEventListener("click", () => void loadBackup());

  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") {
      const hadSession = Boolean(token);
      endSession(hadSession ? "Viewer session cleared because the page was hidden." : "", hadSession);
    }
  });
  window.addEventListener("pagehide", () => {
    const hadSession = Boolean(token);
    endSession(hadSession ? "Viewer session cleared because the page was left." : "", hadSession);
  });
  window.addEventListener("pageshow", (event) => {
    if (event.persisted || token) {
      endSession("Viewer session cleared. Generate a new one-time session to continue.", false);
    } else {
      clearPrivateData();
      presentLogin();
    }
  });

  clearPrivateData();
  presentLogin();
})();
