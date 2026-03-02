<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { FitAddon } from "@xterm/addon-fit";
  import { SearchAddon } from "@xterm/addon-search";
  import { SerializeAddon } from "@xterm/addon-serialize";
  import { Unicode11Addon } from "@xterm/addon-unicode11";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import { WebglAddon } from "@xterm/addon-webgl";
  import { Terminal } from "xterm";

  import type {
    CreateSessionResponse,
    LifecyclePreferences,
    ProfileInfo,
    PtyErrorEvent,
    PtyExitedEvent,
    PtyOutputEvent,
    SessionInfo,
    SessionSnapshot,
    WorkspaceState,
  } from "./lib/types";

  let terminalHost: HTMLDivElement | null = null;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let searchAddon: SearchAddon | null = null;
  let serializeAddon: SerializeAddon | null = null;
  let resizeObserver: ResizeObserver | null = null;

  let profiles: ProfileInfo[] = [];
  let sessions: SessionInfo[] = [];
  let activeProfileId: string | null = null;
  let activeSessionId: string | null = null;
  let activeSessionSeq = 0;
  let activeSnapshotNeedsReconnectBreak = false;
  let renamingSessionId: string | null = null;
  let renameDraft = "";
  let renameBusy = false;
  let lastError = "";
  let sessionSearch = "";
  let profileMenuOpen = false;
  let creatingProfile = false;
  let newProfileDraft = "";
  let renameProfileDraft = "";
  let renameProfileBusy = false;
  let deletingProfileBusy = false;
  let profileMenuHost: HTMLDivElement | null = null;

  let unlistenOutput: UnlistenFn | null = null;
  let unlistenExited: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;
  let unlistenTrayNewSession: UnlistenFn | null = null;

  const TERMINAL_SCROLLBACK = 1000;
  const MAX_SERIALIZED_CACHE_SESSIONS = 24;
  const activationInFlight = new Map<string, Promise<void>>();
  const localInputBufferBySession = new Map<string, string>();
  const sessionRenderedSeqById = new Map<string, number>();
  const sessionLiveSeqById = new Map<string, number>();
  const serializedSnapshotBySession = new Map<string, string>();
  let renderedSessionId: string | null = null;
  let searchQuery = "";
  let lifecyclePreferences: LifecyclePreferences = {
    keep_alive_on_close: true,
    start_in_tray: false,
  };
  let lifecyclePreferencesBusy = false;

  $: activeSession =
    sessions.find((session) => session.session_id === activeSessionId) ?? null;

  $: activeProfile =
    profiles.find((profile) => profile.profile_id === activeProfileId) ??
    profiles[0] ??
    null;

  $: filteredSessions = sessions.filter((session) => {
    const needle = sessionSearch.trim().toLowerCase();
    if (!needle) {
      return true;
    }
    const haystack = `${session.name} ${session.cwd}`.toLowerCase();
    return haystack.includes(needle);
  });

  function primeSessionSeqMap(source: SessionInfo[]) {
    for (const session of source) {
      if (session.seq === 0) {
        sessionLiveSeqById.set(session.session_id, 0);
        sessionRenderedSeqById.set(session.session_id, 0);
        serializedSnapshotBySession.delete(session.session_id);
        continue;
      }
      sessionLiveSeqById.set(session.session_id, session.seq);
      if (!sessionRenderedSeqById.has(session.session_id) && session.seq > 0) {
        sessionRenderedSeqById.set(session.session_id, session.seq);
      }
    }
  }

  function updateRenderedSeq(sessionId: string, seq: number) {
    if (seq <= 0) {
      return;
    }
    const current = sessionRenderedSeqById.get(sessionId) ?? 0;
    if (seq > current) {
      sessionRenderedSeqById.set(sessionId, seq);
    }
  }

  function setSerializedCache(sessionId: string, serialized: string) {
    if (!serialized.trim()) {
      return;
    }

    if (serializedSnapshotBySession.has(sessionId)) {
      serializedSnapshotBySession.delete(sessionId);
    }
    serializedSnapshotBySession.set(sessionId, serialized);

    while (serializedSnapshotBySession.size > MAX_SERIALIZED_CACHE_SESSIONS) {
      const oldest = serializedSnapshotBySession.keys().next().value;
      if (!oldest) {
        break;
      }
      serializedSnapshotBySession.delete(oldest);
    }
  }

  function captureActiveTerminalSnapshot() {
    if (!terminal || !serializeAddon || !activeSessionId || renderedSessionId !== activeSessionId) {
      return;
    }

    try {
      const serialized = serializeAddon.serialize({ scrollback: TERMINAL_SCROLLBACK });
      setSerializedCache(activeSessionId, serialized);
      updateRenderedSeq(activeSessionId, activeSessionSeq);
    } catch (_error) {
      // Serialize failure should not block runtime.
    }
  }

  function restoreTerminalFromSerializedCache(sessionId: string, liveSeq: number): boolean {
    const serialized = serializedSnapshotBySession.get(sessionId);
    const cachedSeq = sessionRenderedSeqById.get(sessionId) ?? 0;
    if (!serialized || cachedSeq < liveSeq || !terminal || liveSeq <= 0) {
      return false;
    }

    terminal.reset();
    terminal.write(serialized);
    renderedSessionId = sessionId;
    activeSessionSeq = cachedSeq;
    activeSnapshotNeedsReconnectBreak = !serialized.endsWith("\n") && !serialized.endsWith("\r");
    return true;
  }

  function safeOpenExternalLink(uri: string) {
    try {
      const url = new URL(uri);
      if (url.protocol !== "http:" && url.protocol !== "https:") {
        return;
      }
      window.open(url.toString(), "_blank", "noopener,noreferrer");
    } catch (_error) {
      // Ignore invalid URIs from terminal output.
    }
  }

  function onGlobalKeydown(event: KeyboardEvent) {
    if (!searchAddon) {
      return;
    }

    const target = event.target as HTMLElement | null;
    if (
      target &&
      (target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.tagName === "SELECT" ||
        target.isContentEditable)
    ) {
      return;
    }

    const key = event.key.toLowerCase();
    if ((event.ctrlKey || event.metaKey) && key === "f") {
      event.preventDefault();
      const next = window.prompt("Find in terminal", searchQuery);
      if (!next) {
        return;
      }
      searchQuery = next;
      searchAddon.findNext(searchQuery, { caseSensitive: false });
      return;
    }

    if (event.key === "F3" && searchQuery.trim().length > 0) {
      event.preventDefault();
      if (event.shiftKey) {
        searchAddon.findPrevious(searchQuery, { caseSensitive: false });
      } else {
        searchAddon.findNext(searchQuery, { caseSensitive: false });
      }
    }
  }

  function sessionTone(session: SessionInfo) {
    const subject = `${session.name} ${session.cwd}`.toLowerCase();
    if (subject.includes("prod") || subject.includes("server")) {
      return "tone-indigo";
    }
    if (subject.includes("db") || subject.includes("postgre") || subject.includes("mysql")) {
      return "tone-emerald";
    }
    if (subject.includes("staging") || subject.includes("offline") || session.status === "disconnected") {
      return "tone-rose";
    }
    if (subject.includes("local") || subject.includes("dev")) {
      return "tone-amber";
    }
    return "tone-slate";
  }

  function sessionGlyph(session: SessionInfo) {
    const normalized = session.name.trim();
    if (!normalized) {
      return "#";
    }

    const words = normalized.split(/\s+/).filter(Boolean);
    if (words.length === 1) {
      return words[0].slice(0, 2).toUpperCase();
    }

    return `${words[0][0] ?? ""}${words[1][0] ?? ""}`.toUpperCase();
  }

  function statusLabel(session: SessionInfo) {
    return session.status === "running" ? "SSH Connected" : "Disconnected";
  }

  function profileGlyph(profile: ProfileInfo | null) {
    if (!profile) {
      return "P";
    }

    const normalized = profile.name.trim();
    if (!normalized) {
      return "P";
    }

    const words = normalized.split(/\s+/).filter(Boolean);
    if (words.length === 1) {
      return words[0].slice(0, 2).toUpperCase();
    }

    return `${words[0][0] ?? ""}${words[1][0] ?? ""}`.toUpperCase();
  }

  async function listSessions() {
    sessions = await invoke<SessionInfo[]>("list_sessions");
    primeSessionSeqMap(sessions);
  }

  async function applyWorkspace(workspace: WorkspaceState) {
    captureActiveTerminalSnapshot();
    profiles = workspace.profiles;
    activeProfileId = workspace.active_profile_id ?? profiles[0]?.profile_id ?? null;
    sessions = workspace.sessions;
    primeSessionSeqMap(sessions);
    activeSessionId = workspace.active_session_id ?? sessions[0]?.session_id ?? null;
    activeSessionSeq = 0;
    renderedSessionId = null;
    activationInFlight.clear();

    if (
      renamingSessionId &&
      sessions.every((session) => session.session_id !== renamingSessionId)
    ) {
      cancelRename();
    }

    await hydrateActiveSession();
    await resizeActiveSession();
  }

  async function loadWorkspaceState() {
    const workspace = await invoke<WorkspaceState>("load_workspace");
    await applyWorkspace(workspace);
  }

  async function loadLifecyclePreferences() {
    try {
      lifecyclePreferences = await invoke<LifecyclePreferences>("get_lifecycle_preferences");
    } catch (error) {
      lastError = `get_lifecycle_preferences failed: ${String(error)}`;
    }
  }

  async function setLifecyclePreferences(next: Partial<LifecyclePreferences>) {
    if (lifecyclePreferencesBusy) {
      return;
    }

    lifecyclePreferencesBusy = true;
    try {
      lifecyclePreferences = await invoke<LifecyclePreferences>("set_lifecycle_preferences", {
        payload: {
          keep_alive_on_close: next.keep_alive_on_close,
          start_in_tray: next.start_in_tray,
        },
      });
      lastError = "";
    } catch (error) {
      lastError = `set_lifecycle_preferences failed: ${String(error)}`;
    } finally {
      lifecyclePreferencesBusy = false;
    }
  }

  function toggleProfileMenu() {
    profileMenuOpen = !profileMenuOpen;
    if (profileMenuOpen) {
      renameProfileDraft = activeProfile?.name ?? "";
    }
  }

  function closeProfileMenu() {
    profileMenuOpen = false;
  }

  async function switchProfile(profileId: string) {
    if (!profileId || profileId === activeProfileId) {
      closeProfileMenu();
      return;
    }

    try {
      const workspace = await invoke<WorkspaceState>("switch_profile", {
        payload: { profile_id: profileId },
      });
      await applyWorkspace(workspace);
      closeProfileMenu();
      await resizeActiveSession();
    } catch (error) {
      lastError = `switch_profile failed: ${String(error)}`;
    }
  }

  async function createProfile() {
    if (creatingProfile) {
      return;
    }

    creatingProfile = true;
    try {
      const draft = newProfileDraft.trim();
      const created = await invoke<ProfileInfo>("create_profile", {
        payload: {
          name: draft.length > 0 ? draft : null,
        },
      });
      newProfileDraft = "";
      await switchProfile(created.profile_id);
    } catch (error) {
      lastError = `create_profile failed: ${String(error)}`;
    } finally {
      creatingProfile = false;
    }
  }

  async function renameActiveProfile() {
    if (renameProfileBusy || !activeProfileId) {
      return;
    }

    const trimmed = renameProfileDraft.trim();
    if (!trimmed) {
      lastError = "Profile name cannot be empty";
      return;
    }

    renameProfileBusy = true;
    try {
      await invoke<ProfileInfo>("rename_profile", {
        payload: {
          profile_id: activeProfileId,
          name: trimmed,
        },
      });
      await loadWorkspaceState();
      renameProfileDraft = trimmed;
    } catch (error) {
      lastError = `rename_profile failed: ${String(error)}`;
    } finally {
      renameProfileBusy = false;
    }
  }

  async function deleteActiveProfile() {
    if (deletingProfileBusy || !activeProfileId) {
      return;
    }

    if (profiles.length <= 1) {
      lastError = "Cannot delete the last profile";
      return;
    }

    const profileName = activeProfile?.name ?? "this profile";
    const approved = window.confirm(
      `Delete profile \"${profileName}\"? This will close and remove all sessions in this profile.`,
    );
    if (!approved) {
      return;
    }

    deletingProfileBusy = true;
    try {
      const workspace = await invoke<WorkspaceState>("delete_profile", {
        payload: { profile_id: activeProfileId },
      });
      await applyWorkspace(workspace);
      closeProfileMenu();
      await resizeActiveSession();
    } catch (error) {
      lastError = `delete_profile failed: ${String(error)}`;
    } finally {
      deletingProfileBusy = false;
    }
  }

  function onProfileCreateKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      void createProfile();
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeProfileMenu();
    }
  }

  function onProfileRenameKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      void renameActiveProfile();
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      renameProfileDraft = activeProfile?.name ?? "";
      closeProfileMenu();
    }
  }

  function onDocumentPointerDown(event: MouseEvent) {
    if (!profileMenuOpen) {
      return;
    }
    const target = event.target as Node | null;
    if (profileMenuHost && target && profileMenuHost.contains(target)) {
      return;
    }
    closeProfileMenu();
  }

  function getSession(sessionId: string | null) {
    if (!sessionId) {
      return null;
    }
    return sessions.find((session) => session.session_id === sessionId) ?? null;
  }

  function isSessionRunning(sessionId: string | null) {
    return getSession(sessionId)?.status === "running";
  }

  async function createSession() {
    const { cols, rows } = getTerminalSize();
    const response = await invoke<CreateSessionResponse>("create_session", {
      payload: { cols, rows },
    });

    await listSessions();
    await setActiveSession(response.session_id, { connect: false });
    await resizeActiveSession();
  }

  async function closeSession(sessionId: string) {
    await invoke("close_session", {
      payload: { session_id: sessionId },
    });
    localInputBufferBySession.delete(sessionId);
    sessionRenderedSeqById.delete(sessionId);
    sessionLiveSeqById.delete(sessionId);
    serializedSnapshotBySession.delete(sessionId);
    if (renderedSessionId === sessionId) {
      renderedSessionId = null;
    }

    if (renamingSessionId === sessionId) {
      cancelRename();
    }

    await listSessions();

    if (activeSessionId === sessionId) {
      const next = sessions[0] ?? null;
      activeSessionId = next?.session_id ?? null;
      activeSessionSeq = 0;
      await hydrateActiveSession();
      await resizeActiveSession();
    }
  }

  async function ensureSessionConnected(
    sessionId: string,
    options: { forceActivate?: boolean } = {},
  ): Promise<boolean> {
    const session = getSession(sessionId);
    if (!session) {
      return false;
    }

    const shouldActivate =
      options.forceActivate === true || session.status !== "running";
    if (!shouldActivate) {
      return true;
    }

    let pending = activationInFlight.get(sessionId);
    if (!pending) {
      const { cols, rows } = getTerminalSize();

      if (
        session.status !== "running" &&
        activeSessionId === sessionId &&
        activeSnapshotNeedsReconnectBreak &&
        terminal
      ) {
        terminal.write("\r\n");
        activeSnapshotNeedsReconnectBreak = false;
      }

      pending = (async () => {
        await invoke<void>("activate_session", {
          payload: {
            session_id: sessionId,
            cols,
            rows,
          },
        });
        await listSessions();
      })();
      activationInFlight.set(sessionId, pending);
    }

    try {
      await pending;
      return true;
    } catch (error) {
      lastError = `activate_session failed: ${String(error)}`;
      await listSessions();
      return false;
    } finally {
      if (activationInFlight.get(sessionId) === pending) {
        activationInFlight.delete(sessionId);
      }
    }
  }

  async function setActiveSession(
    sessionId: string,
    options: { connect?: boolean } = {},
  ) {
    if (activeSessionId === sessionId) {
      if (options.connect ?? true) {
        await ensureSessionConnected(sessionId);
      }
      await resizeActiveSession();
      return;
    }

    captureActiveTerminalSnapshot();
    activeSessionId = sessionId;
    activeSessionSeq = 0;

    await hydrateActiveSession();

    if (options.connect ?? true) {
      const connected = await ensureSessionConnected(sessionId);
      if (!connected) {
        return;
      }
    }

    await resizeActiveSession();
  }

  function startRename(session: SessionInfo) {
    if (renameBusy) {
      return;
    }
    renamingSessionId = session.session_id;
    renameDraft = session.name;
  }

  function cancelRename() {
    renamingSessionId = null;
    renameDraft = "";
  }

  async function renameSession(sessionId: string) {
    if (renameBusy || renamingSessionId !== sessionId) {
      return;
    }

    const trimmedName = renameDraft.trim();
    if (!trimmedName) {
      lastError = "Session name cannot be empty";
      return;
    }

    renameBusy = true;
    try {
      await invoke("rename_session", {
        payload: {
          session_id: sessionId,
          name: trimmedName,
        },
      });
      await listSessions();
      cancelRename();
    } catch (error) {
      lastError = `rename_session failed: ${String(error)}`;
      await listSessions();
    } finally {
      renameBusy = false;
    }
  }

  async function setSessionPersist(sessionId: string, persistHistory: boolean) {
    try {
      await invoke<void>("set_session_persist", {
        payload: {
          session_id: sessionId,
          persist_history: persistHistory,
        },
      });
      await listSessions();
    } catch (error) {
      lastError = `set_session_persist failed: ${String(error)}`;
    }
  }

  async function toggleActivePersist() {
    if (!activeSession) {
      return;
    }
    await setSessionPersist(activeSession.session_id, !activeSession.persist_history);
  }

  async function clearActiveSessionHistory() {
    if (!activeSessionId) {
      return;
    }

    try {
      await invoke<void>("clear_session_history", {
        payload: { session_id: activeSessionId },
      });
      activeSessionSeq = 0;
      sessionRenderedSeqById.set(activeSessionId, 0);
      sessionLiveSeqById.set(activeSessionId, 0);
      serializedSnapshotBySession.delete(activeSessionId);
      terminal?.reset();
      renderedSessionId = activeSessionId;
      await hydrateActiveSession();
    } catch (error) {
      lastError = `clear_session_history failed: ${String(error)}`;
    }
  }

  async function clearAllHistory() {
    try {
      await invoke<void>("clear_all_history");
      activeSessionSeq = 0;
      sessionRenderedSeqById.clear();
      sessionLiveSeqById.clear();
      serializedSnapshotBySession.clear();
      terminal?.reset();
      renderedSessionId = activeSessionId;
      await hydrateActiveSession();
      localInputBufferBySession.clear();
    } catch (error) {
      lastError = `clear_all_history failed: ${String(error)}`;
    }
  }

  async function tryHandleLocalSlashCommand(sessionId: string, data: string): Promise<boolean> {
    if (!terminal || terminal.buffer.active.type !== "normal") {
      return false;
    }

    let buffer = localInputBufferBySession.get(sessionId) ?? "";
    let shouldClearHistory = false;

    for (const char of data) {
      if (char === "\r" || char === "\n") {
        const command = buffer.trim();
        if (command === "clear") {
          shouldClearHistory = true;
        }
        buffer = "";
        continue;
      }

      if (char === "\u007f" || char === "\b") {
        buffer = buffer.slice(0, -1);
        continue;
      }

      if (char < " ") {
        continue;
      }

      buffer += char;
      if (buffer.length > 256) {
        buffer = buffer.slice(-256);
      }
    }

    localInputBufferBySession.set(sessionId, buffer);

    if (!shouldClearHistory) {
      return false;
    }

    try {
      await invoke<void>("clear_session_history", {
        payload: { session_id: sessionId },
      });
      activeSessionSeq = 0;
      sessionRenderedSeqById.set(sessionId, 0);
      sessionLiveSeqById.set(sessionId, 0);
      serializedSnapshotBySession.delete(sessionId);
      lastError = "";
    } catch (error) {
      lastError = `clear_session_history failed: ${String(error)}`;
    }

    return false;
  }

  async function activateActiveSession() {
    if (!activeSessionId) {
      return;
    }

    const connected = await ensureSessionConnected(activeSessionId);
    if (!connected) {
      return;
    }

    await resizeActiveSession();
  }

  async function hydrateActiveSession() {
    if (!terminal) {
      return;
    }

    const requestedSessionId = activeSessionId;
    if (!requestedSessionId) {
      if (renderedSessionId !== null) {
        terminal.reset();
      }
      activeSessionSeq = 0;
      activeSnapshotNeedsReconnectBreak = false;
      renderedSessionId = null;
      return;
    }

    const listedSeq =
      sessions.find((session) => session.session_id === requestedSessionId)?.seq ?? 0;
    const liveSeq = Math.max(listedSeq, sessionLiveSeqById.get(requestedSessionId) ?? 0);
    sessionLiveSeqById.set(requestedSessionId, liveSeq);

    if (renderedSessionId === requestedSessionId && activeSessionSeq >= liveSeq) {
      return;
    }

    if (restoreTerminalFromSerializedCache(requestedSessionId, liveSeq)) {
      return;
    }

    let snapshot: SessionSnapshot;
    try {
      snapshot = await invoke<SessionSnapshot>("get_session_snapshot", {
        payload: {
          session_id: requestedSessionId,
          preview_lines: TERMINAL_SCROLLBACK,
        },
      });
    } catch (error) {
      const errorText = String(error);
      const sessionMissing = errorText.toLowerCase().includes("session not found");
      await listSessions();
      if (
        activeSessionId === requestedSessionId &&
        sessions.every((session) => session.session_id !== requestedSessionId)
      ) {
        activeSessionId = sessions[0]?.session_id ?? null;
        activeSessionSeq = 0;
        if (activeSessionId) {
          await hydrateActiveSession();
        }
      }

      if (!sessionMissing) {
        lastError = `get_session_snapshot failed: ${errorText}`;
      } else {
        lastError = "";
      }
      activeSnapshotNeedsReconnectBreak = false;
      return;
    }

    if (!terminal || activeSessionId !== requestedSessionId) {
      return;
    }

    terminal.reset();
    renderedSessionId = requestedSessionId;
    activeSessionSeq = snapshot.seq;
    sessionLiveSeqById.set(requestedSessionId, snapshot.seq);
    updateRenderedSeq(requestedSessionId, snapshot.seq);
    activeSnapshotNeedsReconnectBreak =
      snapshot.content.length > 0 &&
      !snapshot.content.endsWith("\n") &&
      !snapshot.content.endsWith("\r");
    if (snapshot.content.length > 0) {
      terminal.write(snapshot.content);
      setSerializedCache(requestedSessionId, snapshot.content);
    }
  }

  function getTerminalSize() {
    return {
      cols: terminal?.cols ?? 120,
      rows: terminal?.rows ?? 32,
    };
  }

  async function resizeActiveSession() {
    if (!activeSessionId || !terminal || !fitAddon) {
      return;
    }

    fitAddon.fit();
    const { cols, rows } = getTerminalSize();

    if (!isSessionRunning(activeSessionId)) {
      return;
    }

    try {
      await invoke<void>("resize_session", {
        payload: {
          session_id: activeSessionId,
          cols,
          rows,
        },
      });
    } catch (error) {
      lastError = `resize_session failed: ${String(error)}`;
    }
  }

  async function bootstrapTerminal() {
    if (!terminalHost) {
      return;
    }

    terminal = new Terminal({
      allowProposedApi: true,
      scrollback: TERMINAL_SCROLLBACK,
      fontFamily:
        '"Noto Sans Mono", "Noto Mono", "DejaVu Sans Mono", "Ubuntu Mono", "JetBrains Mono", monospace',
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);

    try {
      const unicode11Addon = new Unicode11Addon();
      terminal.loadAddon(unicode11Addon);
      terminal.unicode.activeVersion = "11";
    } catch (_error) {
      // Keep default unicode provider if addon unavailable.
    }

    try {
      searchAddon = new SearchAddon();
      terminal.loadAddon(searchAddon);
    } catch (_error) {
      searchAddon = null;
    }

    try {
      serializeAddon = new SerializeAddon();
      terminal.loadAddon(serializeAddon);
    } catch (_error) {
      serializeAddon = null;
    }

    try {
      const webLinksAddon = new WebLinksAddon((_event, uri) => {
        safeOpenExternalLink(uri);
      });
      terminal.loadAddon(webLinksAddon);
    } catch (_error) {
      // Continue without link detection.
    }

    try {
      const webgl = new WebglAddon();
      terminal.loadAddon(webgl);
    } catch (_error) {
      // Fallback silently to canvas renderer.
    }

    terminal.open(terminalHost);
    fitAddon.fit();

    terminal.onData(async (data) => {
      if (!activeSessionId) {
        return;
      }

      const sessionId = activeSessionId;
      const localCommandHandled = await tryHandleLocalSlashCommand(sessionId, data);
      if (localCommandHandled) {
        return;
      }

      const connected = await ensureSessionConnected(sessionId);
      if (!connected) {
        return;
      }

      try {
        await invoke<void>("write_input", {
          payload: {
            session_id: sessionId,
            data,
          },
        });
      } catch (error) {
        lastError = `write_input failed: ${String(error)}`;
      }
    });

    resizeObserver = new ResizeObserver(() => {
      void resizeActiveSession();
    });
    resizeObserver.observe(terminalHost);
  }

  async function setupEventListeners() {
    unlistenOutput = await listen<PtyOutputEvent>("pty/output", ({ payload }) => {
      sessionLiveSeqById.set(payload.session_id, payload.seq);

      if (!terminal) {
        return;
      }

      if (payload.session_id !== activeSessionId) {
        const cachedSeq = sessionRenderedSeqById.get(payload.session_id) ?? 0;
        if (payload.seq > cachedSeq) {
          serializedSnapshotBySession.delete(payload.session_id);
        }
        return;
      }

      if (payload.seq <= activeSessionSeq) {
        return;
      }

      activeSessionSeq = payload.seq;
      renderedSessionId = payload.session_id;
      updateRenderedSeq(payload.session_id, payload.seq);
      terminal.write(payload.chunk);
    });

    unlistenExited = await listen<PtyExitedEvent>("pty/exited", ({ payload }) => {
      void syncSessionsAfterExit(payload.session_id);
    });

    unlistenError = await listen<PtyErrorEvent>("pty/error", ({ payload }) => {
      lastError = `[${payload.session_id}] ${payload.message}`;
    });

    unlistenTrayNewSession = await listen("app/tray-new-session", async () => {
      try {
        await createSession();
      } catch (error) {
        lastError = `tray new session failed: ${String(error)}`;
      }
    });
  }

  function onSessionKeydown(event: KeyboardEvent, sessionId: string) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      void setActiveSession(sessionId);
    }
  }

  function onRenameInputKeydown(event: KeyboardEvent, sessionId: string) {
    event.stopPropagation();

    if (renameBusy) {
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      void renameSession(sessionId);
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      cancelRename();
    }
  }

  async function syncSessionsAfterExit(exitedSessionId: string) {
    await listSessions();

    if (activeSessionId === exitedSessionId) {
      await hydrateActiveSession();
    }

    if (activeSessionId && sessions.every((session) => session.session_id !== activeSessionId)) {
      activeSessionId = sessions[0]?.session_id ?? null;
      activeSessionSeq = 0;
      await hydrateActiveSession();
    }
  }

  onMount(async () => {
    document.addEventListener("mousedown", onDocumentPointerDown, true);
    document.addEventListener("keydown", onGlobalKeydown, true);
    await bootstrapTerminal();
    await setupEventListeners();
    await loadLifecyclePreferences();

    await loadWorkspaceState();
    if (sessions.length === 0) {
      await createSession();
    }
  });

  onDestroy(() => {
    document.removeEventListener("mousedown", onDocumentPointerDown, true);
    document.removeEventListener("keydown", onGlobalKeydown, true);
    captureActiveTerminalSnapshot();
    resizeObserver?.disconnect();
    unlistenOutput?.();
    unlistenExited?.();
    unlistenError?.();
    unlistenTrayNewSession?.();
    terminal?.dispose();
    activationInFlight.clear();
  });
</script>

<div class="termi-layout">
  <aside class="termi-sidebar">
    <div class="brand-header">
      <div class="brand-copy">
        <div class="brand-icon">⌘</div>
        <div>
          <h1>TermiChat</h1>
          <p><span class="online-dot"></span> Online</p>
        </div>
      </div>
      <button class="new-session-btn" on:click={() => void createSession()}>New</button>
    </div>

    <div class="search-box-wrap">
      <span class="search-icon">⌕</span>
      <input bind:value={sessionSearch} class="search-box" placeholder="Search sessions..." />
    </div>

    <div class="sessions-panel">
      <p class="section-title">Active Sessions</p>

      <ul class="session-list">
        {#if filteredSessions.length === 0}
          <li class="session-empty">No sessions match your search.</li>
        {/if}

        {#each filteredSessions as session (session.session_id)}
          <li>
            <div
              class={`session-card ${session.session_id === activeSessionId ? "active" : ""}`}
              role="button"
              tabindex="0"
              on:click={() => void setActiveSession(session.session_id)}
              on:keydown={(event) => onSessionKeydown(event, session.session_id)}
            >
              <div class={`session-avatar ${sessionTone(session)}`}>{sessionGlyph(session)}</div>

              <div class="session-body">
                {#if renamingSessionId === session.session_id}
                  <input
                    class="rename-input"
                    bind:value={renameDraft}
                    disabled={renameBusy}
                    maxlength="120"
                    on:click|stopPropagation
                    on:keydown|stopPropagation={(event) =>
                      onRenameInputKeydown(event, session.session_id)}
                  />
                {:else}
                  <p class="session-name">{session.name}</p>
                  <p class="session-sub" title={session.cwd}>{session.cwd}</p>
                {/if}

                <p class={`session-state ${session.status}`}>{statusLabel(session)}</p>
              </div>

              <div class="session-actions">
                {#if renamingSessionId === session.session_id}
                  <button
                    class="mini-btn"
                    disabled={renameBusy}
                    on:click|stopPropagation={() => void renameSession(session.session_id)}
                    on:keydown|stopPropagation
                  >
                    Save
                  </button>
                  <button
                    class="mini-btn"
                    disabled={renameBusy}
                    on:click|stopPropagation={cancelRename}
                    on:keydown|stopPropagation
                  >
                    Cancel
                  </button>
                {:else}
                  <button
                    class="mini-btn"
                    on:click|stopPropagation={() =>
                      void setSessionPersist(session.session_id, !session.persist_history)}
                  >
                    {session.persist_history ? "Save on" : "Save off"}
                  </button>
                  <button class="mini-btn" on:click|stopPropagation={() => startRename(session)}>
                    Rename
                  </button>
                  <button
                    class="mini-btn danger"
                    on:click|stopPropagation={() => void closeSession(session.session_id)}
                  >
                    ×
                  </button>
                {/if}
              </div>
            </div>
          </li>
        {/each}
      </ul>
    </div>

    <div class="profile-footer" bind:this={profileMenuHost}>
      <button
        class="profile-avatar profile-avatar-btn"
        title="Switch profile"
        on:click={toggleProfileMenu}
      >
        {profileGlyph(activeProfile)}
      </button>
      <button class="profile-copy profile-copy-btn" on:click={toggleProfileMenu}>
        <p>{activeProfile?.name ?? "No profile"}</p>
        <span>{profiles.length} profile(s)</span>
      </button>
      <button class="mini-btn profile-menu-toggle" title="Profiles" on:click={toggleProfileMenu}>
        ▾
      </button>

      {#if profileMenuOpen}
        <div class="profile-menu">
          <p class="profile-menu-title">Profiles</p>
          <div class="profile-menu-list">
            {#each profiles as profile (profile.profile_id)}
              <button
                class={`profile-menu-item ${profile.profile_id === activeProfileId ? "active" : ""}`}
                on:click={() => void switchProfile(profile.profile_id)}
              >
                <span class="profile-menu-name">{profile.name}</span>
                {#if profile.profile_id === activeProfileId}
                  <span class="profile-menu-badge">Current</span>
                {/if}
              </button>
            {/each}
          </div>
          <div class="profile-rename-row">
            <input
              class="profile-create-input"
              placeholder="Rename current profile"
              bind:value={renameProfileDraft}
              maxlength="80"
              on:keydown={onProfileRenameKeydown}
            />
            <button
              class="mini-btn"
              disabled={renameProfileBusy || !activeProfileId}
              on:click={() => void renameActiveProfile()}
            >
              {renameProfileBusy ? "..." : "Rename"}
            </button>
          </div>
          <div class="profile-create-row">
            <input
              class="profile-create-input"
              placeholder="New profile name"
              bind:value={newProfileDraft}
              maxlength="80"
              on:keydown={onProfileCreateKeydown}
            />
            <button class="mini-btn" disabled={creatingProfile} on:click={() => void createProfile()}>
              {creatingProfile ? "..." : "Create"}
            </button>
          </div>
          <div class="profile-pref-row">
            <label class="profile-pref-label">
              <input
                type="checkbox"
                checked={lifecyclePreferences.keep_alive_on_close}
                disabled={lifecyclePreferencesBusy}
                on:change={(event) =>
                  void setLifecyclePreferences({
                    keep_alive_on_close: (event.currentTarget as HTMLInputElement).checked,
                  })}
              />
              Keep running in tray when close
            </label>
          </div>
          <div class="profile-pref-row">
            <label class="profile-pref-label">
              <input
                type="checkbox"
                checked={lifecyclePreferences.start_in_tray}
                disabled={lifecyclePreferencesBusy}
                on:change={(event) =>
                  void setLifecyclePreferences({
                    start_in_tray: (event.currentTarget as HTMLInputElement).checked,
                  })}
              />
              Start in tray
            </label>
          </div>
          <button
            class="mini-btn danger profile-delete-btn"
            disabled={deletingProfileBusy || profiles.length <= 1 || !activeProfileId}
            on:click={() => void deleteActiveProfile()}
          >
            {deletingProfileBusy ? "..." : "Delete active profile"}
          </button>
        </div>
      {/if}
    </div>
  </aside>

  <main class="terminal-shell">
    <div class="terminal-header">
      <div class="terminal-meta">
        <span>
          {activeSession ? `Active: ${activeSession.name} (${activeSession.status})` : "No active session"}
        </span>
        {#if activeSession}
          <span class="terminal-cwd" title={activeSession.cwd}>{activeSession.cwd}</span>
        {/if}
      </div>
      <div class="terminal-actions-row">
        {#if activeSession && activeSession.status === "disconnected"}
          <button class="header-btn" on:click={() => void activateActiveSession()}>Reconnect</button>
        {/if}
        {#if activeSession}
          <button class="header-btn" on:click={() => void toggleActivePersist()}>
            {activeSession.persist_history ? "Persist: on" : "Persist: off"}
          </button>
          <button class="header-btn" on:click={() => void clearActiveSessionHistory()}>
            Clear session
          </button>
        {/if}
        <button class="header-btn" on:click={() => void clearAllHistory()}>Clear all</button>
      </div>
    </div>

    {#if lastError}
      <div class="error-bar">{lastError}</div>
    {/if}

    <div class="terminal-pane">
      {#if sessions.length === 0}
        <div class="empty-state">No sessions yet</div>
      {/if}

      <div class="terminal-host" bind:this={terminalHost}></div>

      {#if activeSession && activeSession.status === "disconnected"}
        <div class="terminal-overlay">
          Session disconnected. Click Reconnect hoặc gõ lệnh để spawn lại PTY.
        </div>
      {/if}
    </div>
  </main>
</div>
