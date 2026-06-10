import { describe, expect, test } from "bun:test";
import { createSignal } from "solid-js";
import { TextAttributes } from "@opentui/core";
import { testRender } from "@opentui/solid";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { deflateSync } from "node:zlib";
import { EntryList, entryMouseSelectionOptions } from "./components/EntryList";
import { HeaderBar } from "./components/HeaderBar";
import { PreviewPane } from "./components/PreviewPane";
import { Shell } from "./components/Shell";
import { StatusLine } from "./components/StatusLine";
import { ModeOverlay } from "./components/Overlay";
import { FullPreview } from "./components/FullPreview";
import { AppFrame } from "./App";
import { initialState } from "./state";
import { resolveTuiConfig, surface } from "./tui-config";
import type { Entry } from "./types";

describe("OpenTUI render snapshots", () => {
  test("maps mouse row gestures to selection intents", () => {
    expect(entryMouseSelectionOptions({ button: 0 })).toEqual({ extend: false, toggle: false });
    expect(entryMouseSelectionOptions({ button: 0, modifiers: { shift: true } })).toEqual({ extend: true, toggle: false });
    expect(entryMouseSelectionOptions({ button: 0, modifiers: { ctrl: true } })).toEqual({ extend: false, toggle: true });
    expect(entryMouseSelectionOptions({ button: 2 })).toEqual({ extend: false, toggle: true });
    expect(entryMouseSelectionOptions({ button: "right" })).toEqual({ extend: false, toggle: true });
    expect(entryMouseSelectionOptions({ button: 1 })).toBeNull();
  });

  test("renders the main shell with history, preview, and status surfaces", async () => {
    const config = resolveTuiConfig({
      labels: {
        brand: "DX",
        ready: "ready",
      },
      chrome: {
        selectedMarker: ">",
        markedMarker: "*",
        normalMarker: " ",
        statusSeparator: "|",
      },
      layout: {
        listWidthPercent: 44,
        maxPreviewLines: 8,
        showMetadata: true,
        imagePreviewMode: "metadata",
      },
    });
    const entries: Entry[] = [
      textEntry(1, "hello from ditox", true),
      imageEntry(2),
      textEntry(3, "second clipboard item", false),
    ];
    const state = {
      ...initialState(),
      entries,
      selectedIndex: 0,
      selectedIds: new Set([2]),
      status: "copied",
      watcher: {
        running: true,
        paused: false,
        backend: "wayland",
        poll_interval_ms: 750,
        last_seen_ms: Date.now(),
        last_error: null,
      },
    };

    const view = await testRender(
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={state} selectedCount={state.selectedIds.size} />
          <box flexGrow={1} flexDirection="row">
            <EntryList
              config={config}
              entries={entries}
              selectedIndex={state.selectedIndex}
              selectedIds={state.selectedIds}
              rows={8}
              width={38}
              query={state.query}
              onSelectEntry={() => {}}
              onScroll={() => {}}
            />
            <PreviewPane config={config} entry={entries[state.selectedIndex]} rows={8} width={46} />
          </box>
          <StatusLine config={config} status={state.status} watcher={state.watcher} width={90} />
        </Shell>
      ),
      { width: 90, height: 18 },
    );

    await view.renderOnce();
    const frame = view.captureCharFrame();
    view.renderer.destroy();

    expect(frame).toContain("DX");
    expect(frame).toContain("history");
    expect(frame).toContain("preview");
    expect(frame).toContain("> TXT");
    expect(frame).toContain("* IMG");
    expect(frame).toContain("hello from ditox");
    expect(frame).toContain("watcher live");
    expect(frame).toContain("copied");
  });

  test("aligns header and status line content from layout config", async () => {
    const config = resolveTuiConfig({
      labels: {
        brand: "HDR",
        headerLineTemplate: "{brand}",
        statusLineTemplate: "{operation}",
      },
      chrome: {
        headerBorder: false,
      },
      layout: {
        headerHeight: 1,
        statusHeight: 1,
        headerPaddingX: 0,
        statusPaddingX: 0,
        headerContentAlign: "center",
        statusContentAlign: "right",
      },
    });
    const state = initialState();
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={state} selectedCount={0} />
          <box flexGrow={1} />
          <StatusLine config={config} status="STS" watcher={null} width={20} />
        </Shell>
      ),
      20,
      4,
    );

    expect(columnOf(frame, "HDR")).toBe(8);
    expect(columnOf(frame, "STS")).toBe(17);
  });

  test("can vertically align taller header and status bars", async () => {
    const config = resolveTuiConfig({
      labels: {
        brand: "HDR",
        headerLineTemplate: "{brand}",
        statusLineTemplate: "{operation}",
      },
      chrome: {
        headerBorder: false,
      },
      layout: {
        headerHeight: 3,
        statusHeight: 3,
        headerPaddingX: 0,
        headerPaddingY: 0,
        statusPaddingX: 0,
        statusPaddingY: 0,
        headerVerticalAlign: "bottom",
        statusVerticalAlign: "center",
      },
    });
    const state = initialState();
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={state} selectedCount={0} />
          <box flexGrow={1} />
          <StatusLine config={config} status="STS" watcher={null} width={20} />
        </Shell>
      ),
      20,
      8,
    );

    expect(lineIndex(frame, "HDR")).toBe(2);
    expect(lineIndex(frame, "STS")).toBe(6);
  });

  test("renders optional status line chrome", async () => {
    const config = resolveTuiConfig({
      labels: {
        statusTitle: "STATE",
        statusLineTemplate: "{operation}",
      },
      chrome: {
        statusBorder: true,
        showStatusTitle: true,
        statusBorderStyle: "single",
        statusTitleAlignment: "right",
      },
      layout: {
        statusHeight: 3,
        statusPaddingX: 1,
        statusPaddingY: 0,
      },
      styles: {
        status: { border: "#abcdef" },
      },
    });
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <StatusLine config={config} status="READY" watcher={null} width={28} />
        </Shell>
      ),
      32,
      5,
    );
    const spans = await captureSpans(
      () => (
        <Shell config={config}>
          <StatusLine config={config} status="READY" watcher={null} width={28} />
        </Shell>
      ),
      32,
      5,
    );

    expect(frame).toContain("STATE");
    expect(frame).toContain("READY");
    expect(colorHex(findSpan(spans, "STATE"))).toBe("#abcdef");
  });

  test("applies per-surface semantic color overrides in rendered spans", async () => {
    const config = resolveTuiConfig({
      labels: {
        pinnedViewTitle: "SAVED",
        searchPrompt: "SEARCH>",
      },
      styles: {
        header: { search: "#123456", favorite: "#654321", secondary: "#336699" },
        overlay: { search: "#aa00cc" },
        status: { success: "#118855" },
      },
      statusTones: {
        success: ["archived"],
      },
    });
    const entries = [textEntry(1, "needle value", true)];
    const state = {
      ...initialState(),
      entries,
      selectedIndex: 0,
      selectedIds: new Set([1]),
      pinnedOnly: true,
      query: "needle",
      mode: "search" as const,
      status: "archived",
      watcher: null,
    };

    const spans = await captureSpans(
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={state} selectedCount={state.selectedIds.size} />
          <StatusLine config={config} status={state.status} watcher={state.watcher} width={80} />
          <ModeOverlay config={config} state={state} />
        </Shell>
      ),
      80,
      9,
    );

    expect(colorHex(findSpan(spans, "SAVED"))).toBe("#654321");
    expect(colorHex(findSpan(spans, "needle"))).toBe("#123456");
    expect(colorHex(findSpan(spans, "selected 1"))).toBe("#336699");
    expect(colorHex(findSpan(spans, "SEARCH>"))).toBe("#aa00cc");
    expect(colorHex(findSpan(spans, "archived"))).toBe("#118855");
  });

  test("applies per-surface text attributes in rendered spans", async () => {
    const config = resolveTuiConfig({
      labels: {
        ready: "dim-ready",
        searchPrompt: "find:",
      },
      styles: {
        selectedRow: { bold: true, underline: true },
        status: { dim: true },
        overlay: { inverse: true },
      },
      layout: {
        showRowMetadata: false,
      },
    });
    const entries = [textEntry(1, "attribute clip", false)];
    const state = {
      ...initialState(),
      entries,
      selectedIndex: 0,
      mode: "search" as const,
      query: "attr",
      watcher: null,
    };

    const spans = await captureSpans(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={entries}
            selectedIndex={state.selectedIndex}
            selectedIds={state.selectedIds}
            rows={4}
            width={60}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
          <StatusLine config={config} status={state.status} watcher={state.watcher} width={80} />
          <ModeOverlay config={config} state={state} />
        </Shell>
      ),
      80,
      12,
    );

    expect(hasAttributes(findSpan(spans, "attribute clip"), TextAttributes.BOLD | TextAttributes.UNDERLINE)).toBe(true);
    expect(hasAttributes(findSpan(spans, "dim-ready"), TextAttributes.DIM)).toBe(true);
    expect(hasAttributes(findSpan(spans, "find:attr"), TextAttributes.INVERSE)).toBe(true);
  });

  test("renders configurable header and status line templates", async () => {
    const config = resolveTuiConfig({
      labels: {
        brand: "DX",
        pinnedViewTitle: "SAVED",
        headerLineTemplate: "{query} <- {brand} [{filter}] {mode}",
        statusHintTemplate: "{searchKeys}:{search}",
        statusLineTemplate: "{operation}{separator}{watcher}{separator}{hint}",
      },
      chrome: {
        statusSeparator: "/",
      },
      layout: {
        statusSeparatorPaddingLeft: 1,
        statusSeparatorPaddingRight: 2,
      },
      styles: {
        header: { search: "#123456", favorite: "#654321" },
        status: { success: "#118855" },
      },
    });
    const state = {
      ...initialState(),
      entries: [textEntry(1, "needle value", true)],
      selectedIndex: 0,
      selectedIds: new Set([1, 2]),
      pinnedOnly: true,
      query: "needle",
      status: "copied",
      watcher: {
        running: true,
        paused: false,
        backend: "wayland",
        poll_interval_ms: 750,
        last_seen_ms: Date.now(),
        last_error: null,
      },
    };

    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={state} selectedCount={state.selectedIds.size} />
          <StatusLine config={config} status={state.status} watcher={state.watcher} width={90} />
        </Shell>
      ),
      90,
      6,
    );

    expect(frame).toContain("needle <- DX [SAVED] selected 2");
    expect(frame).toContain("copied /  watcher live");
    expect(frame).toContain(" /  /:search");

    const spans = await captureSpans(
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={state} selectedCount={state.selectedIds.size} />
          <StatusLine config={config} status={state.status} watcher={state.watcher} width={90} />
        </Shell>
      ),
      90,
      6,
    );
    expect(colorHex(findSpan(spans, "needle"))).toBe("#123456");
    expect(colorHex(findSpan(spans, "SAVED"))).toBe("#654321");
    expect(colorHex(findSpan(spans, "copied"))).toBe("#118855");
  });

  test("bounds header segments in narrow terminals", async () => {
    const config = resolveTuiConfig({
      labels: {
        brand: "DITOX-BRAND-LONG",
        headerLineTemplate: "{brand}|{filter}|{query}|{mode}",
      },
      chrome: {
        headerBorder: false,
      },
      layout: {
        headerHeight: 1,
        headerPaddingX: 0,
        headerBrandMaxWidth: 5,
        headerFilterMaxWidth: 4,
        headerQueryMaxWidth: 8,
        headerModeMaxWidth: 6,
      },
    });
    const state = {
      ...initialState(),
      filter: "images" as const,
      query: "verylongsearchquery",
    };

    const frame = await captureFrame(
      () => <HeaderBar config={config} state={state} selectedCount={3} width={24} />,
      24,
      1,
    );

    expect(frame).toContain("DI...|I...|ver...|sel...");
    expect(frame).not.toContain("DITOX-BRAND-LONG");
    expect(frame).not.toContain("verylongsearchquery");
    expect(frame.split("\n")[0]?.length).toBe(24);
  });

  test("bounds status line segments in narrow terminals", async () => {
    const config = resolveTuiConfig({
      labels: {
        statusHintTemplate: "{pasteKeys}:{paste}{separator}{copyKeys}:{copy}{separator}{searchKeys}:{search}",
        statusLineTemplate: "{operation}{separator}{watcher}{separator}{hint}",
      },
      chrome: {
        statusSeparator: "|",
      },
      layout: {
        statusPaddingX: 0,
        statusSeparatorPadding: 0,
        statusOperationMaxWidth: 9,
        statusWatcherMaxWidth: 8,
        statusHintMaxWidth: 7,
      },
    });

    const frame = await captureFrame(
      () => <StatusLine config={config} status="operation-message-that-is-too-wide" watcher={null} width={28} />,
      28,
      1,
    );

    expect(frame).toContain("operat...");
    expect(frame).toContain("watch...");
    expect(frame).toContain("...");
    expect(frame).not.toContain("operation-message-that-is-too-wide");
    expect(frame).not.toContain("watcher stopped");
    expect(frame.split("\n")[0]?.length).toBe(28);
  });

  test("bounds overlay copy in narrow terminals", async () => {
    const config = resolveTuiConfig({
      labels: {
        searchInputTemplate: "{prompt}{query}{cursor}",
        searchPrompt: "search-prompt:",
        searchCursor: "cursor-long",
        deleteOneTemplate: "{message}",
        deleteOne: "delete prompt that cannot fit",
        confirmHintTemplate: "{hint}",
        confirmHint: "confirmation hint that cannot fit",
        helpMoveSelection: "move selection action that cannot fit",
      },
      chrome: {
        overlayBorder: false,
      },
      layout: {
        searchOverlayHeight: 1,
        confirmOverlayHeight: 2,
        helpOverlayHeight: 8,
        searchOverlayPaddingX: 0,
        dangerOverlayPaddingX: 0,
        helpOverlayPaddingX: 0,
        searchOverlayPromptMaxWidth: 6,
        searchOverlayQueryMaxWidth: 8,
        searchOverlayCursorMaxWidth: 4,
        dangerOverlayPromptMaxWidth: 10,
        dangerOverlayHintMaxWidth: 9,
        helpOverlayActionMaxWidth: 12,
        helpKeyWidth: 8,
      },
    });

    const searchFrame = await captureFrame(
      () => <ModeOverlay config={config} state={{ ...initialState(), mode: "search" as const, query: "verylongquery" }} width={20} />,
      20,
      1,
    );
    expect(searchFrame).toContain("sea...");
    expect(searchFrame).toContain("veryl...");
    expect(searchFrame).toContain("c...");
    expect(searchFrame).not.toContain("search-prompt");
    expect(searchFrame).not.toContain("verylongquery");

    const deleteFrame = await captureFrame(
      () => <ModeOverlay config={config} state={{ ...initialState(), mode: "confirm-delete" as const }} width={20} />,
      20,
      2,
    );
    expect(deleteFrame).toContain("delete ...");
    expect(deleteFrame).toContain("confir...");
    expect(deleteFrame).not.toContain("delete prompt that cannot fit");
    expect(deleteFrame).not.toContain("confirmation hint that cannot fit");

    const helpFrame = await captureFrame(
      () => <ModeOverlay config={config} state={{ ...initialState(), mode: "help" as const }} width={22} />,
      22,
      8,
    );
    expect(helpFrame).toContain("move sele...");
    expect(helpFrame).not.toContain("move selection action that cannot fit");
    expectFrameWithin(searchFrame, 20, 1);
    expectFrameWithin(deleteFrame, 20, 2);
    expectFrameWithin(helpFrame, 22, 8);
  });

  test("fits the default help keymap inside its frame", async () => {
    const config = resolveTuiConfig();

    const frame = await captureFrame(
      () => <ModeOverlay config={config} state={{ ...initialState(), mode: "help" as const }} width={126} />,
      126,
      config.layout.helpOverlayHeight,
    );

    expect(frame).toContain("clear including pinned");
    expect(frame).toContain("c x");
    expectFrameWithin(frame, 126, config.layout.helpOverlayHeight);
  });

  test("does not paint extra help rows past a short configured frame", async () => {
    const config = resolveTuiConfig({
      layout: {
        helpOverlayHeight: 8,
      },
    });

    const frame = await captureFrame(
      () => <ModeOverlay config={config} state={{ ...initialState(), mode: "help" as const }} width={80} />,
      80,
      8,
    );

    expect(frame).toContain("move selection");
    expect(frame).not.toContain("clear everything");
    expectFrameWithin(frame, 80, 8);
  });

  test("routes status line placeholder tones from config", async () => {
    const config = resolveTuiConfig({
      labels: {
        statusHintTemplate: "HELP-HINT",
        statusLineTemplate: "{operation}{separator}{watcher}{separator}{hint}",
      },
      chrome: {
        statusSeparator: "|",
      },
      layout: {
        statusSeparatorPadding: 0,
      },
      styles: {
        status: {
          success: "#11aa66",
          warning: "#cc9900",
          secondary: "#6699cc",
          accent: "#ff66aa",
        },
      },
      statusLineTones: {
        operation: "success",
        watcher: "warning",
        hint: "secondary",
        separator: "accent",
      },
    });
    const watcher = {
      running: true,
      paused: true,
      backend: "wayland",
      poll_interval_ms: 750,
      last_seen_ms: Date.now(),
      last_error: null,
    };

    const spans = await captureSpans(
      () => (
        <Shell config={config}>
          <StatusLine config={config} status="stored" watcher={watcher} width={70} />
        </Shell>
      ),
      70,
      3,
    );

    expect(colorHex(findSpan(spans, "stored"))).toBe("#11aa66");
    expect(colorHex(findSpan(spans, "watcher paused"))).toBe("#cc9900");
    expect(colorHex(findSpan(spans, "HELP-HINT"))).toBe("#6699cc");
    expect(colorHex(findSpan(spans, "|"))).toBe("#ff66aa");
  });

  test("renders mode-aware status hints from file config", async () => {
    const config = resolveTuiConfig({
      labels: {
        statusLineTemplate: "{hint}",
        statusHintSeparator: " | ",
        statusSearchModeHintTemplate: "{applyKeys}:{apply}{separator}{backspaceKeys}:{backspace}{separator}{cancelKeys}:{cancel}",
        statusPreviewModeHintTemplate: "{previewBackKeys}:{previewBack}{separator}{previewScrollKeys}:{previewScroll}",
        statusConfirmModeHintTemplate: "{confirmYesKeys}:{confirmYes}{separator}{confirmNoKeys}:{confirmNo}",
        statusApplyHint: "go",
        statusBackspaceHint: "erase",
        statusCancelHint: "stop",
        statusPreviewBackHint: "return",
        statusPreviewScrollHint: "move",
        statusConfirmYesHint: "commit",
        statusConfirmNoHint: "abort",
      },
      keyBindings: {
        searchApply: "ret",
        searchBackspace: "bs",
        searchCancel: "esc",
        previewBack: "esc",
        previewUp: "k",
        previewDown: "j",
        previewPageUp: "u",
        previewPageDown: "d",
        confirmYes: "yes",
        confirmNo: "no",
      },
    });

    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <StatusLine config={config} status="" watcher={null} width={90} mode="search" />
          <StatusLine config={config} status="" watcher={null} width={90} mode="preview" />
          <StatusLine config={config} status="" watcher={null} width={90} mode="confirm" />
        </Shell>
      ),
      90,
      5,
    );

    expect(frame).toContain("ret:go | bs:erase | esc:stop");
    expect(frame).toContain("esc:return | k j u d:move");
    expect(frame).toContain("yes:commit | no:abort");
  });

  test("routes header line placeholder tones from config", async () => {
    const config = resolveTuiConfig({
      labels: {
        brand: "BRAND",
        filterLabel: "FLT",
        queryLabel: "QRY",
        modeLabel: "MODE",
        headerSectionSeparator: "SEP",
        headerLabelSeparator: "EQ",
        pinnedViewTitle: "PINNED",
        headerLineTemplate: "{brand}{sectionSeparator}{filter}{labelSeparator}{query}{mode}{filterLabel}{queryLabel}{modeLabel}",
      },
      styles: {
        header: {
          fg: "#eeeeee",
          muted: "#777777",
          success: "#11aa66",
          warning: "#cc9900",
          search: "#22aadd",
          secondary: "#6699cc",
          accent: "#ff66aa",
          image: "#aa66ff",
          favorite: "#f2c14e",
        },
      },
      headerLineTones: {
        brand: "success",
        filter: "warning",
        query: "search",
        mode: "secondary",
        filterLabel: "accent",
        queryLabel: "image",
        modeLabel: "favorite",
        sectionSeparator: "fg",
        labelSeparator: "muted",
      },
    });
    const state = {
      ...initialState(),
      pinnedOnly: true,
      query: "needle",
    };

    const spans = await captureSpans(
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={state} selectedCount={0} />
        </Shell>
      ),
      90,
      5,
    );

    expect(colorHex(findSpan(spans, "BRAND"))).toBe("#11aa66");
    expect(colorHex(findSpan(spans, "PINNED"))).toBe("#cc9900");
    expect(colorHex(findSpan(spans, "needle"))).toBe("#22aadd");
    expect(colorHex(findSpan(spans, "single"))).toBe("#6699cc");
    expect(colorHex(findSpan(spans, "FLT"))).toBe("#ff66aa");
    expect(colorHex(findSpan(spans, "QRY"))).toBe("#aa66ff");
    expect(colorHex(findSpan(spans, "MODE"))).toBe("#f2c14e");
    expect(colorHex(findSpan(spans, "SEP"))).toBe("#eeeeee");
    expect(colorHex(findSpan(spans, "EQ"))).toBe("#777777");
  });

  test("highlights literal search matches in row previews with configurable row colors", async () => {
    const config = resolveTuiConfig({
      styles: {
        selectedRow: { search: "#ff66cc" },
        list: { search: "#22ccff" },
      },
      layout: {
        listWidthPercent: 68,
        rowPreviewReservedWidth: 8,
      },
    });
    const entries = [
      textEntry(1, "alpha needle beta", false),
      textEntry(2, "second needle row", false),
    ];
    const spans = await captureSpans(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={entries}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={3}
            width={54}
            query="needle"
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      90,
      7,
    );

    expect(colorHex(findSpan(spans, "needle"))).toBe("#ff66cc");
  });

  test("can disable search-match row highlighting while keeping query text visible", async () => {
    const config = resolveTuiConfig({
      styles: {
        selectedRow: { fg: "#eeeeee", search: "#ff66cc" },
      },
      layout: {
        highlightSearchMatches: false,
        listWidthPercent: 68,
        rowPreviewReservedWidth: 8,
      },
    });
    const spans = await captureSpans(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[textEntry(1, "alpha needle beta", false)]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={3}
            width={54}
            query="needle"
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      90,
      7,
    );

    expect(colorHex(findSpan(spans, "needle"))).toBe("#eeeeee");
  });

  test("routes list content tones from config", async () => {
    const config = resolveTuiConfig({
      labels: {
        rowMetaTemplate: "META",
        noMatchesTitle: "EMPTY-TITLE",
        noMatchesHelp: "EMPTY-HELP",
      },
      chrome: {
        selectedMarker: "S!",
        normalMarker: "N!",
        scrollbarThumb: "TH",
        scrollbarTrack: "TR",
      },
      layout: {
        rowMarkerGap: 1,
        rowMetaPreviewGap: 1,
        rowPreviewReservedWidth: 10,
        rowPreviewMaxWidth: 30,
        scrollbarWidth: 2,
      },
      styles: {
        selectedRow: {
          success: "#11aa66",
          warning: "#cc9900",
          secondary: "#6699cc",
          favorite: "#f2c14e",
        },
        emptyState: {
          image: "#aa66ff",
          accent: "#ff66aa",
        },
        scrollbar: {
          error: "#ff3355",
          border: "#445566",
        },
      },
      listContentTones: {
        marker: "success",
        metadata: "warning",
        preview: "secondary",
        searchMatch: "favorite",
        emptyTitle: "image",
        emptyHelp: "accent",
        scrollbarThumb: "error",
        scrollbarTrack: "border",
      },
    });

    const rowSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[textEntry(1, "alpha needle beta", false)]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={3}
            width={72}
            widthPercent={100}
            query="needle"
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      80,
      6,
    );
    expect(colorHex(findSpan(rowSpans, "S!"))).toBe("#11aa66");
    expect(colorHex(findSpan(rowSpans, "META"))).toBe("#cc9900");
    expect(colorHex(findSpan(rowSpans, "alpha"))).toBe("#6699cc");
    expect(colorHex(findSpan(rowSpans, "needle"))).toBe("#f2c14e");

    const emptySpans = await captureSpans(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={3}
            width={72}
            widthPercent={100}
            query="missing"
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      80,
      6,
    );
    expect(colorHex(findSpan(emptySpans, "EMPTY-TITLE"))).toBe("#aa66ff");
    expect(colorHex(findSpan(emptySpans, "EMPTY-HELP"))).toBe("#ff66aa");

    const scrollbarSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[
              textEntry(1, "one", false),
              textEntry(2, "two", false),
              textEntry(3, "three", false),
              textEntry(4, "four", false),
              textEntry(5, "five", false),
              textEntry(6, "six", false),
            ]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={4}
            width={72}
            widthPercent={100}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      80,
      7,
    );
    expect(colorHex(findSpan(scrollbarSpans, "TH"))).toBe("#ff3355");
    expect(colorHex(findSpan(scrollbarSpans, "TR"))).toBe("#445566");
  });

  test("renders selected marked rows with their own configurable surface", async () => {
    const config = resolveTuiConfig({
      chrome: {
        selectedMarker: ">",
        selectedMarkedMarker: "SM",
        markedMarker: "+",
        normalMarker: "|",
      },
      layout: {
        showRowMetadata: false,
        rowMarkerGap: 1,
        rowPreviewMaxWidth: 40,
      },
      styles: {
        selectedMarkedRow: {
          bg: "#203040",
          fg: "#ddeeff",
          accent: "#ffcc00",
          search: "#ff66aa",
          bold: true,
        },
      },
    });
    const spans = await captureSpans(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[textEntry(1, "alpha selected marked", false)]}
            selectedIndex={0}
            selectedIds={new Set([1])}
            rows={3}
            width={54}
            widthPercent={100}
            query="marked"
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      64,
      6,
    );

    expect(colorHex(findSpan(spans, "SM"))).toBe("#ffcc00");
    expect(colorHex(findSpan(spans, "SM"), "bg")).toBe("#203040");
    expect(colorHex(findSpan(spans, "alpha"))).toBe("#ddeeff");
    expect(colorHex(findSpan(spans, "marked"))).toBe("#ff66aa");
    expect(hasAttributes(findSpan(spans, "alpha"), TextAttributes.BOLD)).toBe(true);
  });

  test("renders alternate history rows with their own configurable surface", async () => {
    const config = resolveTuiConfig({
      styles: {
        alternateRow: { bg: "#102030", fg: "#ddeeff", accent: "#66ccaa" },
      },
      layout: {
        alternateRows: true,
        rowPreviewReservedWidth: 8,
      },
    });
    const spans = await captureSpans(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[textEntry(1, "first plain row", false), textEntry(2, "zebra", false), textEntry(3, "third plain row", false)]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={4}
            width={48}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      80,
      8,
    );

    const alternate = findSpan(spans, "zebra");
    expect(colorHex(alternate)).toBe("#ddeeff");
    expect(colorHex(alternate, "bg")).toBe("#102030");
  });

  test("renders customized search and delete overlays", async () => {
    const config = resolveTuiConfig({
      labels: {
        searchTitle: "find",
        searchPrompt: "find> ",
        searchInputTemplate: "{prompt}[{query}]",
        deleteTitle: "remove",
        deleteManyTemplate: "delete {count} clips",
        deletePinnedWarning: "saved item included",
        confirmHint: "yes/no",
        confirmHintTemplate: "action: {hint}",
      },
    });
    const entries = [textEntry(1, "saved value", true), textEntry(2, "second saved value", false)];
    const state = {
      ...initialState(),
      entries,
      selectedIndex: 0,
      selectedIds: new Set([1, 2]),
      query: "saved",
      mode: "search" as const,
    };

    const searchFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={state} />
        </Shell>
      ),
      72,
      8,
    );
    expect(searchFrame).toContain("find");
    expect(searchFrame).toContain("find> [saved]");

    const deleteFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...state, mode: "confirm-delete" }} />
        </Shell>
      ),
      72,
      8,
    );
    expect(deleteFrame).toContain("remove");
    expect(deleteFrame).toContain("delete 2 clips");
    expect(deleteFrame).toContain("saved item included");
    expect(deleteFrame).toContain("action: yes/no");
  });

  test("can place mode overlays below the header or at the bottom", async () => {
    const state = { ...initialState(), mode: "search" as const, query: "needle" };
    const topConfig = resolveTuiConfig({ layout: { overlayPlacement: "top" } });
    const bottomConfig = resolveTuiConfig({ layout: { overlayPlacement: "bottom" } });

    const topFrame = await captureFrame(
      () => (
        <AppFrame
          config={topConfig}
          header={<box height={1}><text>HEADER</text></box>}
          content={<box height={1}><text>CONTENT</text></box>}
          status={<box height={1}><text>STATUS</text></box>}
          overlay={<ModeOverlay config={topConfig} state={state} />}
        />
      ),
      72,
      8,
    );
    expect(lineIndex(topFrame, "HEADER")).toBeLessThan(lineIndex(topFrame, "search"));
    expect(lineIndex(topFrame, "search")).toBeLessThan(lineIndex(topFrame, "CONTENT"));
    expect(lineIndex(topFrame, "CONTENT")).toBeLessThan(lineIndex(topFrame, "STATUS"));

    const bottomFrame = await captureFrame(
      () => (
        <AppFrame
          config={bottomConfig}
          header={<box height={1}><text>HEADER</text></box>}
          content={<box height={1}><text>CONTENT</text></box>}
          status={<box height={1}><text>STATUS</text></box>}
          overlay={<ModeOverlay config={bottomConfig} state={state} />}
        />
      ),
      72,
      8,
    );
    expect(lineIndex(bottomFrame, "HEADER")).toBeLessThan(lineIndex(bottomFrame, "CONTENT"));
    expect(lineIndex(bottomFrame, "CONTENT")).toBeLessThan(lineIndex(bottomFrame, "STATUS"));
    expect(lineIndex(bottomFrame, "STATUS")).toBeLessThan(lineIndex(bottomFrame, "search"));
  });

  test("renders shell padding as an outer TUI inset", async () => {
    const config = resolveTuiConfig({ layout: { shellPaddingX: 2, shellPaddingY: 1 } });
    const frame = await captureFrame(
      () => (
        <AppFrame
          config={config}
          content={
            <box height={1}>
              <text>PADDED</text>
            </box>
          }
        />
      ),
      20,
      5,
    );

    expect(lineIndex(frame, "PADDED")).toBe(1);
    expect(columnOf(frame, "PADDED")).toBe(2);
  });

  test("aligns overlay content independently from overlay titles", async () => {
    const config = resolveTuiConfig({
      helpOrder: ["paste"],
      labels: {
        searchInputTemplate: "{query}",
        deleteOneTemplate: "DANGER",
        helpPaste: "HELP",
      },
      keyBindings: {
        copyPaste: "h",
      },
      chrome: {
        searchOverlayBorder: false,
        dangerOverlayBorder: false,
        helpOverlayBorder: false,
      },
      layout: {
        searchOverlayHeight: 1,
        confirmOverlayHeight: 1,
        helpOverlayHeight: 1,
        overlayPaddingX: 0,
        searchOverlayContentAlign: "center",
        dangerOverlayContentAlign: "right",
        helpOverlayContentAlign: "right",
        helpKeyWidth: 1,
      },
    });
    const searchState = { ...initialState(), mode: "search" as const, query: "SEARCH" };

    const searchFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={searchState} />
        </Shell>
      ),
      40,
      3,
    );
    expect(columnOf(searchFrame, "SEARCH")).toBe(17);

    const dangerFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...initialState(), mode: "confirm-delete" as const }} />
        </Shell>
      ),
      40,
      3,
    );
    expect(columnOf(dangerFrame, "DANGER")).toBe(34);

    const helpFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...initialState(), mode: "help" as const }} />
        </Shell>
      ),
      40,
      3,
    );
    expect(columnOf(helpFrame, "HELP")).toBe(36);
  });

  test("can vertically align overlay body content", async () => {
    const config = resolveTuiConfig({
      labels: {
        searchInputTemplate: "{query}",
      },
      chrome: {
        searchOverlayBorder: false,
      },
      layout: {
        searchOverlayHeight: 5,
        searchOverlayPaddingX: 0,
        searchOverlayPaddingY: 0,
        searchOverlayVerticalAlign: "bottom",
      },
    });
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...initialState(), mode: "search" as const, query: "LOW" }} />
        </Shell>
      ),
      24,
      6,
    );

    expect(lineIndex(frame, "LOW")).toBe(4);
    expect(columnOf(frame, "LOW")).toBe(0);
  });

  test("can space multi-line overlay body rows", async () => {
    const config = resolveTuiConfig({
      labels: {
        clearPromptTemplate: "FIRST",
        clearPinnedUnsafeHint: "SECOND",
        confirmHintTemplate: "{hint}",
        confirmHint: "THIRD",
      },
      chrome: {
        dangerOverlayBorder: false,
      },
      layout: {
        clearOverlayHeight: 5,
        dangerOverlayPaddingX: 0,
        dangerOverlayPaddingY: 0,
        dangerOverlayLineSpacing: 1,
      },
    });
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay
            config={config}
            state={{ ...initialState(), mode: "confirm-clear" as const, clearKind: "all", clearPreserveFavorites: false }}
          />
        </Shell>
      ),
      24,
      6,
    );

    expect(lineIndex(frame, "FIRST")).toBe(0);
    expect(lineIndex(frame, "SECOND")).toBe(2);
    expect(lineIndex(frame, "THIRD")).toBe(4);
  });

  test("renders independently configured panel and overlay padding", async () => {
    const entries = [textEntry(1, "PADDED-ROW", false)];
    const state = { ...initialState(), entries, selectedIndex: 0, query: "PADDED-QUERY", mode: "search" as const };
    const config = resolveTuiConfig({
      labels: {
        searchInputTemplate: "{query}",
        deleteOneTemplate: "PADDED-DELETE",
        confirmHintTemplate: "{hint}",
        confirmHint: "PADDED-HINT",
        helpMoveSelection: "PADDED-HELP",
      },
      chrome: {
        panelBorder: false,
        overlayBorder: false,
        selectedMarker: ">",
      },
      layout: {
        panelPaddingX: 0,
        panelPaddingY: 0,
        overlayPaddingX: 0,
        overlayPaddingY: 0,
        listPaddingX: 3,
        listPaddingY: 1,
        previewPaddingX: 2,
        fullPreviewPaddingX: 4,
        searchOverlayPaddingX: 3,
        dangerOverlayPaddingX: 1,
        helpOverlayPaddingX: 2,
        showRowMetadata: false,
        showScrollbar: false,
        showMetadata: false,
        showPreviewGutter: false,
        showFullPreviewMetadata: false,
        showFullPreviewGutter: false,
        rowMarkerGap: 0,
        searchOverlayHeight: 2,
        confirmOverlayHeight: 2,
        helpOverlayHeight: 4,
      },
    });

    const listFrame = await captureFrame(
      () => (
        <EntryList
          config={config}
          entries={entries}
          selectedIndex={0}
          selectedIds={new Set()}
          rows={2}
          width={24}
          widthPercent={100}
          query=""
          onSelectEntry={() => {}}
          onScroll={() => {}}
        />
      ),
      24,
      4,
    );
    expect(lineContaining(listFrame, "PADDED-ROW").startsWith("   >")).toBe(true);

    const previewFrame = await captureFrame(
      () => <PreviewPane config={config} entry={entries[0]} rows={2} width={24} widthPercent={100} />,
      24,
      3,
    );
    expect(lineContaining(previewFrame, "PADDED-ROW").startsWith("  ")).toBe(true);

    const fullFrame = await captureFrame(
      () => <FullPreview config={config} entry={entries[0]} rows={2} width={28} offset={0} onScroll={() => {}} />,
      28,
      3,
    );
    expect(lineContaining(fullFrame, "PADDED-ROW").startsWith("    ")).toBe(true);

    const searchFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={state} />
        </Shell>
      ),
      32,
      4,
    );
    expect(lineContaining(searchFrame, "PADDED-QUERY").startsWith("   ")).toBe(true);

    const deleteFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...state, mode: "confirm-delete" as const }} />
        </Shell>
      ),
      32,
      4,
    );
    expect(lineContaining(deleteFrame, "PADDED-DELETE").startsWith(" ")).toBe(true);

    const helpFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...initialState(), mode: "help" as const }} />
        </Shell>
      ),
      40,
      6,
    );
    expect(lineContaining(helpFrame, "PADDED-HELP").startsWith("  ")).toBe(true);
  });

  test("styles overlay modes independently", async () => {
    const config = resolveTuiConfig({
      labels: {
        searchPrompt: "find:",
        searchInputTemplate: "{prompt}{query}",
        deleteOneTemplate: "remove-single",
        deletePinnedWarning: "warning-single",
        confirmHint: "do-it",
        confirmHintTemplate: "hint {hint}",
        helpMoveSelection: "navigate rows",
      },
      styles: {
        searchOverlay: { search: "#3399ff" },
        dangerOverlay: { error: "#ff3355", warning: "#ffaa33", muted: "#cc8899" },
        helpOverlay: { fg: "#ddeedd", accent: "#55dd88" },
      },
    });
    const entries = [textEntry(1, "saved", true)];
    const baseState = { ...initialState(), entries, selectedIndex: 0, query: "needle" };

    const searchSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...baseState, mode: "search" as const }} />
        </Shell>
      ),
      80,
      8,
    );
    expect(colorHex(findSpan(searchSpans, "find:"))).toBe("#3399ff");
    expect(colorHex(findSpan(searchSpans, "needle"))).toBe("#3399ff");

    const deleteSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...baseState, mode: "confirm-delete" as const }} />
        </Shell>
      ),
      80,
      8,
    );
    expect(colorHex(findSpan(deleteSpans, "remove-single"))).toBe("#ff3355");
    expect(colorHex(findSpan(deleteSpans, "warning-single"))).toBe("#ffaa33");
    expect(colorHex(findSpan(deleteSpans, "hint do-it"))).toBe("#cc8899");

    const helpSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...initialState(), mode: "help" as const }} />
        </Shell>
      ),
      90,
      17,
    );
    expect(colorHex(findSpan(helpSpans, "up"))).toBe("#55dd88");
    expect(colorHex(findSpan(helpSpans, "navigate rows"))).toBe("#ddeedd");
  });

  test("routes overlay border tones from config", async () => {
    const config = resolveTuiConfig({
      labels: {
        searchTitle: "BORDER-SEARCH",
        deleteTitle: "BORDER-DANGER",
        helpTitle: "BORDER-HELP",
      },
      chrome: {
        overlayBorderStyle: "single",
      },
      styles: {
        searchOverlay: { accent: "#11aa66", search: "#22aadd", warning: "#ffaa00" },
        dangerOverlay: { warning: "#cc9900" },
        helpOverlay: { success: "#6699cc" },
      },
      overlayBorderTones: {
        search: "accent",
        danger: "warning",
        command: "success",
      },
    });
    const state = { ...initialState(), entries: [textEntry(1, "saved", true)], selectedIndex: 0, query: "needle" };

    const searchSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...state, mode: "search" as const }} />
        </Shell>
      ),
      80,
      8,
    );
    expect(colorHex(findSpan(searchSpans, "BORDER-SEARCH"))).toBe("#11aa66");

    const deleteSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...state, mode: "confirm-delete" as const }} />
        </Shell>
      ),
      80,
      8,
    );
    expect(colorHex(findSpan(deleteSpans, "BORDER-DANGER"))).toBe("#cc9900");

    const helpSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...initialState(), mode: "help" as const }} />
        </Shell>
      ),
      90,
      17,
    );
    expect(colorHex(findSpan(helpSpans, "BORDER-HELP"))).toBe("#6699cc");
  });

  test("routes overlay content tones from config", async () => {
    const config = resolveTuiConfig({
      labels: {
        searchPrompt: "route-search:",
        searchCursor: "^",
        searchInputTemplate: "{prompt}{query}{cursor}",
        deleteOneTemplate: "route-delete",
        deletePinnedWarning: "route-warning",
        confirmHint: "route-confirm",
        confirmHintTemplate: "{hint}",
        clearPromptTemplate: "route-clear {kind}",
        clearPinnedSafeHint: "route-safe",
        clearPinnedUnsafeHint: "route-unsafe",
        helpMoveSelection: "route-action",
      },
      styles: {
        searchOverlay: { accent: "#11aa66", search: "#22aadd", warning: "#ffaa00" },
        dangerOverlay: {
          fg: "#eeeeee",
          image: "#aa66ff",
          favorite: "#f2c14e",
          warning: "#cc9900",
          secondary: "#6699cc",
          error: "#ff3355",
        },
        helpOverlay: { success: "#00aa66", muted: "#999999" },
      },
      overlayContentTones: {
        searchInput: "accent",
        searchPrompt: "accent",
        searchQuery: "search",
        searchCursor: "warning",
        deletePrompt: "warning",
        deleteWarning: "secondary",
        confirmHint: "fg",
        clearPrompt: "image",
        clearSafeHint: "favorite",
        clearUnsafeHint: "error",
        helpKey: "success",
        helpAction: "muted",
      },
    });
    const state = { ...initialState(), entries: [textEntry(1, "saved", true)], selectedIndex: 0, query: "needle" };

    const searchSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...state, mode: "search" as const }} />
        </Shell>
      ),
      80,
      8,
    );
    expect(colorHex(findSpan(searchSpans, "route-search:"))).toBe("#11aa66");
    expect(colorHex(findSpan(searchSpans, "needle"))).toBe("#22aadd");
    expect(colorHex(findSpan(searchSpans, "^"))).toBe("#ffaa00");

    const deleteSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...state, mode: "confirm-delete" as const }} />
        </Shell>
      ),
      80,
      8,
    );
    expect(colorHex(findSpan(deleteSpans, "route-delete"))).toBe("#cc9900");
    expect(colorHex(findSpan(deleteSpans, "route-warning"))).toBe("#6699cc");
    expect(colorHex(findSpan(deleteSpans, "route-confirm"))).toBe("#eeeeee");

    const clearSafeSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...state, mode: "confirm-clear" as const, clearKind: "all", clearPreserveFavorites: true }} />
        </Shell>
      ),
      80,
      8,
    );
    expect(colorHex(findSpan(clearSafeSpans, "route-clear all"))).toBe("#aa66ff");
    expect(colorHex(findSpan(clearSafeSpans, "route-safe"))).toBe("#f2c14e");

    const clearUnsafeSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...state, mode: "confirm-clear" as const, clearKind: "all", clearPreserveFavorites: false }} />
        </Shell>
      ),
      80,
      8,
    );
    expect(colorHex(findSpan(clearUnsafeSpans, "route-clear all"))).toBe("#aa66ff");
    expect(colorHex(findSpan(clearUnsafeSpans, "route-unsafe"))).toBe("#ff3355");

    const helpSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...initialState(), mode: "help" as const }} />
        </Shell>
      ),
      90,
      17,
    );
    expect(colorHex(findSpan(helpSpans, "up"))).toBe("#00aa66");
    expect(colorHex(findSpan(helpSpans, "route-action"))).toBe("#999999");
  });

  test("renders customized full preview labels and scrolled content", async () => {
    const config = resolveTuiConfig({
      labels: {
        previewModeTitle: "inspect",
        previewBackHint: "back",
        kindText: "STR",
        entryIdPrefix: "id:",
        fullPreviewMetaTemplate: "{kind}::{id}::{mime}",
        fullPreviewBottomTitleTemplate: "clip {id} lines {start}-{end} of {total} :: {back}",
        previewGutterSeparator: ">",
      },
      layout: {
        previewLineNumberWidth: 1,
        previewGutterWidth: 1,
      },
    });
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <FullPreview
            config={config}
            entry={textEntry(9, "line one\nline two\nline three", false)}
            rows={8}
            width={70}
            offset={1}
            onScroll={() => {}}
          />
        </Shell>
      ),
      76,
      10,
    );

    expect(frame).toContain("inspect");
    expect(frame).toContain("STR::9::text/plain");
    expect(frame).toContain("clip 9 lines 2-3 of 3 :: back");
    expect(frame).toContain("2>line two");
    expect(frame).toContain("line two");
    expect(frame).toContain("line three");
    expect(frame).toContain("back");
  });

  test("styles full preview metadata independently from preview content", async () => {
    const config = resolveTuiConfig({
      labels: {
        fullPreviewMetaTemplate: "META-{id}",
      },
      styles: {
        fullPreview: { fg: "#eeeeee" },
        fullPreviewMeta: { accent: "#44aaee" },
      },
    });

    const spans = await captureSpans(
      () => (
        <Shell config={config}>
          <FullPreview
            config={config}
            entry={textEntry(8, "full preview content color stays separate", false)}
            rows={6}
            width={70}
            offset={0}
            onScroll={() => {}}
          />
        </Shell>
      ),
      76,
      8,
    );

    expect(colorHex(findSpan(spans, "META-8"))).toBe("#44aaee");
    expect(colorHex(findSpan(spans, "full preview content color stays separate"))).toBe("#eeeeee");
  });

  test("renders configurable full preview metadata height", async () => {
    const config = resolveTuiConfig({
      labels: {
        fullPreviewMetaTemplate: "META-TALL",
        fullPreviewBottomTitleTemplate: "range {start}-{end}/{total}",
      },
      layout: {
        fullPreviewMetaHeight: 2,
        fullPreviewMetaPaddingX: 2,
        fullPreviewScrollInsetRows: 0,
        showFullPreviewGutter: false,
      },
    });

    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <FullPreview
            config={config}
            entry={textEntry(18, "first body line\nsecond body line\nthird body line\nfourth body line\nfifth body line\nsixth body line", false)}
            rows={7}
            width={52}
            offset={0}
            onScroll={() => {}}
          />
        </Shell>
      ),
      58,
      9,
    );

    expectFrameWithin(frame, 58, 9);
    expect(lineIndex(frame, "first body line") - lineIndex(frame, "META-TALL")).toBe(2);
    expect(frame.replace(/\n$/, "").split("\n").find((line) => line.includes("META-TALL"))).toContain("  META-TALL");
    expect(frame).toContain("range 1-5/6");
    expect(frame).not.toContain("sixth body line");
  });

  test("renders configurable split and full preview metadata padding", async () => {
    const config = resolveTuiConfig({
      labels: {
        previewMetaHeaderTemplate: "SPLIT-META",
        previewMetaDetailsTemplate: "split-details",
        fullPreviewMetaTemplate: "FULL-META",
      },
      chrome: {
        previewBorder: false,
        fullPreviewBorder: false,
      },
      layout: {
        previewMetaHeight: 4,
        previewMetaPaddingX: 2,
        previewMetaPaddingY: 1,
        fullPreviewMetaHeight: 3,
        fullPreviewMetaPaddingX: 2,
        fullPreviewMetaPaddingY: 1,
        showPreviewGutter: false,
        showFullPreviewGutter: false,
        fullPreviewScrollInsetRows: 0,
      },
    });

    const splitFrame = await captureFrame(
      () => <PreviewPane config={config} entry={textEntry(19, "split padded body", false)} rows={6} width={48} widthPercent={100} />,
      48,
      7,
    );
    expect(lineIndex(splitFrame, "SPLIT-META")).toBe(1);
    expect(lineContaining(splitFrame, "SPLIT-META").startsWith("  ")).toBe(true);
    expect(lineIndex(splitFrame, "split padded body") - lineIndex(splitFrame, "SPLIT-META")).toBe(3);

    const fullFrame = await captureFrame(
      () => <FullPreview config={config} entry={textEntry(20, "full padded body", false)} rows={5} width={48} offset={0} onScroll={() => {}} />,
      48,
      6,
    );
    expect(lineIndex(fullFrame, "FULL-META")).toBe(1);
    expect(lineContaining(fullFrame, "FULL-META").startsWith("  ")).toBe(true);
    expect(lineIndex(fullFrame, "full padded body") - lineIndex(fullFrame, "FULL-META")).toBe(2);
  });

  test("can space split and full preview metadata rows", async () => {
    const config = resolveTuiConfig({
      labels: {
        previewMetaHeaderTemplate: "SPLIT-META",
        previewMetaDetailsTemplate: "SPLIT-DETAIL",
        fullPreviewMetaHeaderTemplate: "FULL-META",
        fullPreviewMetaDetailsTemplate: "FULL-DETAIL",
      },
      chrome: {
        previewBorder: false,
        fullPreviewBorder: false,
      },
      layout: {
        previewMetaHeight: 3,
        fullPreviewMetaHeight: 3,
        previewMetaLineSpacing: 1,
        fullPreviewMetaLineSpacing: 1,
        showPreviewGutter: false,
        showFullPreviewGutter: false,
        fullPreviewScrollInsetRows: 0,
      },
    });
    const entry = textEntry(93, "metadata spacing body", false);

    const splitFrame = await captureFrame(
      () => <PreviewPane config={config} entry={entry} rows={5} width={44} widthPercent={100} />,
      44,
      5,
    );
    expect(lineIndex(splitFrame, "SPLIT-DETAIL") - lineIndex(splitFrame, "SPLIT-META")).toBe(2);
    expect(lineIndex(splitFrame, "metadata spacing body") - lineIndex(splitFrame, "SPLIT-DETAIL")).toBe(1);

    const fullFrame = await captureFrame(
      () => <FullPreview config={config} entry={entry} rows={5} width={44} offset={0} onScroll={() => {}} />,
      44,
      5,
    );
    expect(lineIndex(fullFrame, "FULL-DETAIL") - lineIndex(fullFrame, "FULL-META")).toBe(2);
    expect(lineIndex(fullFrame, "metadata spacing body") - lineIndex(fullFrame, "FULL-DETAIL")).toBe(1);
  });

  test("aligns split and full preview content and metadata independently", async () => {
    const config = resolveTuiConfig({
      labels: {
        previewMetaHeaderTemplate: "META",
        previewMetaDetailsTemplate: "DETAIL",
        fullPreviewMetaHeaderTemplate: "FMETA",
        fullPreviewMetaDetailsTemplate: "FDETAIL",
      },
      chrome: {
        previewBorder: false,
        fullPreviewBorder: false,
      },
      layout: {
        showPreviewGutter: false,
        showFullPreviewGutter: false,
        previewPaddingX: 0,
        fullPreviewPaddingX: 0,
        previewMetaPaddingX: 0,
        fullPreviewMetaPaddingX: 0,
        previewMetaHeight: 2,
        fullPreviewMetaHeight: 1,
        fullPreviewScrollInsetRows: 0,
        previewContentAlign: "center",
        fullPreviewContentAlign: "right",
        previewMetaContentAlign: "right",
        fullPreviewMetaContentAlign: "center",
      },
    });
    const entry = textEntry(72, "BODY", false);

    const splitFrame = await captureFrame(
      () => <PreviewPane config={config} entry={entry} rows={4} width={40} widthPercent={100} />,
      40,
      4,
    );
    expect(columnOf(splitFrame, "META")).toBe(36);
    expect(columnOf(splitFrame, "DETAIL")).toBe(34);
    expect(columnOf(splitFrame, "BODY")).toBe(18);

    const fullFrame = await captureFrame(
      () => <FullPreview config={config} entry={entry} rows={3} width={40} offset={0} onScroll={() => {}} />,
      40,
      3,
    );
    expect(columnOf(fullFrame, "FMETA")).toBe(17);
    expect(columnOf(fullFrame, "BODY")).toBe(36);
  });

  test("truncates split preview metadata at narrow width instead of wrapping into the next row", async () => {
    // Regression: at very narrow preview widths the metadata header (which holds
    // the hash) wrapped, bleeding a stray accent-colored fragment onto the detail
    // row. Each metadata line must hard-truncate to the pane width and stay on its
    // own row.
    const config = resolveTuiConfig();
    const entry = textEntry(11, "alpha short clip", false);
    const frame = await captureFrame(
      () => <PreviewPane config={config} entry={entry} rows={6} width={24} widthPercent={100} />,
      24,
      6,
    );
    const lines = frame.replace(/\n$/, "").split("\n");
    // The header line carries kind + id + hash label; with a 12-char hash it must
    // be clipped with the truncation marker rather than wrapping.
    const headerLine = lines.find((line) => line.includes("hash"));
    expect(headerLine, "header line").toBeDefined();
    expect(headerLine!).toContain("...");
    // The mime detail must appear on a different row than the header, and the
    // header row must not contain any of the mime text (no wrap bleed).
    const mimeLine = lines.find((line) => line.includes("text/plain"));
    expect(mimeLine, "mime line").toBeDefined();
    expect(headerLine!).not.toContain("text/plain");
    expect(lines.indexOf(headerLine!)).not.toBe(lines.indexOf(mimeLine!));
    // Every rendered line stays within the pane width (no overflow).
    for (const line of lines) expect(line.length).toBeLessThanOrEqual(24);
  });

  test("can vertically align split and full preview metadata", async () => {
    const config = resolveTuiConfig({
      labels: {
        previewMetaHeaderTemplate: "META",
        previewMetaDetailsTemplate: "DETAIL",
        fullPreviewMetaHeaderTemplate: "FMETA",
        fullPreviewMetaDetailsTemplate: "FDETAIL",
      },
      chrome: {
        previewBorder: false,
        fullPreviewBorder: false,
      },
      layout: {
        showPreviewGutter: false,
        showFullPreviewGutter: false,
        previewMetaPaddingX: 0,
        previewMetaPaddingY: 0,
        fullPreviewMetaPaddingX: 0,
        fullPreviewMetaPaddingY: 0,
        previewMetaHeight: 4,
        fullPreviewMetaHeight: 3,
        previewMetaVerticalAlign: "bottom",
        fullPreviewMetaVerticalAlign: "bottom",
        fullPreviewScrollInsetRows: 0,
      },
    });
    const entry = textEntry(73, "BODY", false);

    const splitFrame = await captureFrame(
      () => <PreviewPane config={config} entry={entry} rows={5} width={32} widthPercent={100} />,
      32,
      5,
    );
    expect(lineIndex(splitFrame, "META")).toBe(2);
    expect(lineIndex(splitFrame, "DETAIL")).toBe(3);
    expect(lineIndex(splitFrame, "BODY")).toBe(4);

    const fullFrame = await captureFrame(
      () => <FullPreview config={config} entry={entry} rows={4} width={32} offset={0} onScroll={() => {}} />,
      32,
      4,
    );
    expect(lineIndex(fullFrame, "FMETA")).toBe(1);
    expect(lineIndex(fullFrame, "FDETAIL")).toBe(2);
    expect(lineIndex(fullFrame, "BODY")).toBe(3);
  });

  test("can vertically align split and full preview body content", async () => {
    const config = resolveTuiConfig({
      chrome: {
        previewBorder: false,
        fullPreviewBorder: false,
      },
      layout: {
        showMetadata: false,
        showFullPreviewMetadata: false,
        showPreviewGutter: false,
        showFullPreviewGutter: false,
        previewPaddingX: 0,
        previewPaddingY: 0,
        fullPreviewPaddingX: 0,
        fullPreviewPaddingY: 0,
        previewBodyVerticalAlign: "bottom",
        fullPreviewBodyVerticalAlign: "bottom",
        fullPreviewScrollInsetRows: 0,
      },
    });
    const entry = textEntry(74, "BODY", false);

    const splitFrame = await captureFrame(
      () => (
        <box height={5} flexDirection="row">
          <PreviewPane config={config} entry={entry} rows={5} width={32} widthPercent={100} />
        </box>
      ),
      32,
      5,
    );
    expect(lineIndex(splitFrame, "BODY")).toBe(4);

    const fullFrame = await captureFrame(
      () => <FullPreview config={config} entry={entry} rows={5} width={32} offset={0} onScroll={() => {}} />,
      32,
      5,
    );
    expect(lineIndex(fullFrame, "BODY")).toBe(4);
  });

  test("styles empty state and preview gutters independently", async () => {
    const config = resolveTuiConfig({
      labels: {
        noHistoryTitle: "empty-title",
        noHistoryHelp: "empty-help",
        previewGutterSeparator: "SG",
        fullPreviewGutterSeparator: "FG",
      },
      layout: {
        showMetadata: false,
        showFullPreviewMetadata: false,
        previewLineNumberWidth: 1,
        previewGutterWidth: 1,
        fullPreviewGutterWidth: 3,
      },
      styles: {
        list: { fg: "#eeeeee" },
        emptyState: { fg: "#22aa66", muted: "#6688aa" },
        preview: { fg: "#ddeeff", muted: "#778899" },
        previewGutter: { muted: "#ffcc00" },
        fullPreview: { fg: "#eeffdd", muted: "#889977" },
        fullPreviewGutter: { muted: "#00ccff" },
      },
    });

    const emptySpans = await captureSpans(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={4}
            width={50}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      58,
      7,
    );
    expect(colorHex(findSpan(emptySpans, "empty-title"))).toBe("#22aa66");
    expect(colorHex(findSpan(emptySpans, "empty-help"))).toBe("#6688aa");

    const previewSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <PreviewPane config={config} entry={textEntry(7, "split gutter content", false)} rows={5} width={60} />
        </Shell>
      ),
      66,
      7,
    );
    expect(colorHex(findSpan(previewSpans, "SG"))).toBe("#ffcc00");
    expect(colorHex(findSpan(previewSpans, "split gutter content"))).toBe("#ddeeff");

    const fullPreviewSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <FullPreview
            config={config}
            entry={textEntry(8, "full gutter content", false)}
            rows={5}
            width={60}
            offset={0}
            onScroll={() => {}}
          />
        </Shell>
      ),
      66,
      7,
    );
    expect(colorHex(findSpan(fullPreviewSpans, "FG"))).toBe("#00ccff");
    expect(colorHex(findSpan(fullPreviewSpans, "full gutter content"))).toBe("#eeffdd");
    const fullPreviewFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <FullPreview
            config={config}
            entry={textEntry(8, "full gutter content", false)}
            rows={5}
            width={60}
            offset={0}
            onScroll={() => {}}
          />
        </Shell>
      ),
      66,
      7,
    );
    expect(fullPreviewFrame).toContain("  1FGfull gutter content");
  });

  test("renders configurable text preview gutter templates", async () => {
    const config = resolveTuiConfig({
      labels: {
        previewTextGutterTemplate: "L{line}",
        previewGutterSeparator: "|",
        fullPreviewGutterSeparator: ":",
      },
      layout: {
        showMetadata: false,
        showFullPreviewMetadata: false,
        previewGutterWidth: 2,
        fullPreviewGutterWidth: 2,
      },
    });

    const splitFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <PreviewPane config={config} entry={textEntry(21, "alpha", false)} rows={4} width={36} />
        </Shell>
      ),
      42,
      6,
    );
    expect(splitFrame).toContain("L1|alpha");

    const fullFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <FullPreview config={config} entry={textEntry(22, "alpha", false)} rows={4} width={36} offset={0} onScroll={() => {}} />
        </Shell>
      ),
      42,
      6,
    );
    expect(fullFrame).toContain("L1:alpha");
  });

  test("routes split preview content tones from config", async () => {
    const config = resolveTuiConfig({
      labels: {
        previewTitle: "SPLIT-PREVIEW",
        previewMetaHeaderTemplate: "SPLIT-META-H",
        previewMetaDetailsTemplate: "SPLIT-META-D",
        previewGutterSeparator: "SPLIT-GAP",
        noEntryTitle: "SPLIT-EMPTY-TITLE",
        noEntryHelp: "SPLIT-EMPTY-HELP",
        imagePreviewFallbackPrefix: "SPLIT-FBP",
        imagePreviewFallbackSeparator: "SPLIT-FBS",
        imagePreviewBlobMissing: "SPLIT-FBR",
      },
      layout: {
        previewLineNumberWidth: 1,
        previewGutterWidth: 1,
        previewMetaHeight: 2,
        imagePreviewMode: "blocks",
      },
      styles: {
        preview: {
          secondary: "#6699cc",
          warning: "#cc9900",
          success: "#11aa66",
          accent: "#ff66aa",
          error: "#ff3355",
        },
        previewGutter: {
          image: "#aa66ff",
          favorite: "#f2c14e",
        },
        previewMeta: {
          warning: "#cc6600",
          border: "#445566",
        },
      },
      previewContentTones: {
        splitBorder: "success",
        splitPrimary: "secondary",
        splitEmptyTitle: "warning",
        splitEmptyHelp: "success",
        splitImageFallbackPrefix: "accent",
        splitImageFallbackSeparator: "success",
        splitImageFallbackReason: "error",
        splitGutter: "image",
        splitGutterSeparator: "favorite",
        splitMetaHeader: "warning",
        splitMetaDetails: "border",
      },
    });

    const textSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <PreviewPane config={config} entry={textEntry(12, "split body", false)} rows={6} width={70} />
        </Shell>
      ),
      76,
      9,
    );
    expect(colorHex(findSpan(textSpans, "SPLIT-PREVIEW"))).toBe("#11aa66");
    expect(colorHex(findSpan(textSpans, "SPLIT-META-H"))).toBe("#cc6600");
    expect(colorHex(findSpan(textSpans, "SPLIT-META-D"))).toBe("#445566");
    expect(colorHex(findSpan(textSpans, "1"))).toBe("#aa66ff");
    expect(colorHex(findSpan(textSpans, "SPLIT-GAP"))).toBe("#f2c14e");
    expect(colorHex(findSpan(textSpans, "split body"))).toBe("#6699cc");

    const emptySpans = await captureSpans(
      () => (
        <Shell config={config}>
          <PreviewPane config={config} entry={undefined} rows={4} width={70} />
        </Shell>
      ),
      76,
      7,
    );
    expect(colorHex(findSpan(emptySpans, "SPLIT-EMPTY-TITLE"))).toBe("#cc9900");
    expect(colorHex(findSpan(emptySpans, "SPLIT-EMPTY-HELP"))).toBe("#11aa66");

    const fallbackSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <PreviewPane config={config} entry={imageEntry(13, null)} rows={4} width={70} />
        </Shell>
      ),
      76,
      12,
    );
    expect(colorHex(findSpan(fallbackSpans, "SPLIT-FBP"))).toBe("#ff66aa");
    expect(colorHex(findSpan(fallbackSpans, "SPLIT-FBS"))).toBe("#11aa66");
    expect(colorHex(findSpan(fallbackSpans, "SPLIT-FBR"))).toBe("#ff3355");
  });

  test("routes full preview content tones from config", async () => {
    const config = resolveTuiConfig({
      labels: {
        previewModeTitle: "FULL-PREVIEW",
        fullPreviewMetaHeaderTemplate: "FULL-META-H",
        fullPreviewMetaDetailsTemplate: "FULL-META-D",
        previewGutterSeparator: "FULL-GAP",
        noEntryTitle: "FULL-EMPTY-TITLE",
        noEntryHelp: "FULL-EMPTY-HELP",
        imagePreviewFallbackPrefix: "FULL-FBP",
        imagePreviewFallbackSeparator: "FULL-FBS",
        imagePreviewBlobMissing: "FULL-FBR",
      },
      layout: {
        previewLineNumberWidth: 1,
        previewGutterWidth: 1,
        imagePreviewMode: "blocks",
        fullPreviewMetaHeight: 2,
      },
      styles: {
        fullPreview: {
          secondary: "#6699cc",
          warning: "#cc9900",
          success: "#11aa66",
          accent: "#ff66aa",
          error: "#ff3355",
        },
        fullPreviewGutter: {
          image: "#aa66ff",
          favorite: "#f2c14e",
        },
        fullPreviewMeta: {
          favorite: "#ffee66",
          secondary: "#88ccff",
        },
      },
      previewContentTones: {
        fullBorder: "success",
        fullMetaHeader: "favorite",
        fullMetaDetails: "secondary",
        fullPrimary: "secondary",
        fullEmptyTitle: "warning",
        fullEmptyHelp: "success",
        fullImageFallbackPrefix: "accent",
        fullImageFallbackSeparator: "success",
        fullImageFallbackReason: "error",
        fullGutter: "image",
        fullGutterSeparator: "favorite",
      },
    });

    const textSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <FullPreview config={config} entry={textEntry(14, "full body", false)} rows={6} width={70} offset={0} onScroll={() => {}} />
        </Shell>
      ),
      76,
      9,
    );
    expect(colorHex(findSpan(textSpans, "FULL-PREVIEW"))).toBe("#11aa66");
    expect(colorHex(findSpan(textSpans, "FULL-META-H"))).toBe("#ffee66");
    expect(colorHex(findSpan(textSpans, "FULL-META-D"))).toBe("#88ccff");
    expect(colorHex(findSpan(textSpans, "1"))).toBe("#aa66ff");
    expect(colorHex(findSpan(textSpans, "FULL-GAP"))).toBe("#f2c14e");
    expect(colorHex(findSpan(textSpans, "full body"))).toBe("#6699cc");

    const emptySpans = await captureSpans(
      () => (
        <Shell config={config}>
          <FullPreview config={config} entry={undefined} rows={5} width={70} offset={0} onScroll={() => {}} />
        </Shell>
      ),
      76,
      8,
    );
    expect(colorHex(findSpan(emptySpans, "FULL-EMPTY-TITLE"))).toBe("#cc9900");
    expect(colorHex(findSpan(emptySpans, "FULL-EMPTY-HELP"))).toBe("#11aa66");

    const fallbackSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <FullPreview config={config} entry={imageEntry(15, null)} rows={5} width={70} offset={0} onScroll={() => {}} />
        </Shell>
      ),
      76,
      8,
    );
    expect(colorHex(findSpan(fallbackSpans, "FULL-FBP"))).toBe("#ff66aa");
    expect(colorHex(findSpan(fallbackSpans, "FULL-FBS"))).toBe("#11aa66");
    expect(colorHex(findSpan(fallbackSpans, "FULL-FBR"))).toBe("#ff3355");
  });

  test("can hide full preview metadata while keeping content visible", async () => {
    const config = resolveTuiConfig({
      labels: {
        previewModeTitle: "reader",
        kindText: "STR",
        fullPreviewMetaTemplate: "hidden {kind} {id}",
      },
      layout: {
        showFullPreviewMetadata: false,
      },
    });
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <FullPreview
            config={config}
            entry={textEntry(4, "first\nsecond", false)}
            rows={6}
            width={70}
            offset={0}
            onScroll={() => {}}
          />
        </Shell>
      ),
      76,
      8,
    );

    expect(frame).toContain("reader");
    expect(frame).toContain("first");
    expect(frame).toContain("second");
    expect(frame).not.toContain("hidden STR 4");
  });

  test("can render compact one-line split preview metadata", async () => {
    const config = resolveTuiConfig({
      labels: {
        kindText: "TXT",
        entryIdPrefix: "clip:",
        previewMetaHeaderTemplate: "{kind} {entryIdPrefix}{id}",
        previewMetaDetailsTemplate: "hidden details {mime}",
      },
      layout: {
        previewMetaHeight: 1,
        imagePreviewMode: "metadata",
      },
    });

    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <PreviewPane config={config} entry={textEntry(9, "metadata compact body", false)} rows={7} width={52} />
        </Shell>
      ),
      62,
      9,
    );

    expect(frame).toContain("TXT clip:9");
    expect(frame).not.toContain("hidden details");
    expect(frame).toContain("metadata compact body");
  });

  test("renders extended preview metadata template placeholders", async () => {
    const config = resolveTuiConfig({
      labels: {
        kindImage: "PIC",
        pinned: "saved",
        previewMetaHeaderTemplate: "{kind}|{id}|{sourceApp}|{dimensions}|{hashShort}",
        previewMetaDetailsTemplate: "{size}|{blob}{pinnedSuffix}",
        fullPreviewMetaTemplate: "{kind}|{id}|{sourceApp}|{size}|{hashShort}{pinnedSuffix}",
        previewPinnedSuffixTemplate: "|{pinnedRaw}",
      },
      layout: {
        imagePreviewMode: "metadata",
        previewMetaHashLength: 6,
        fullPreviewMetaHashLength: 10,
      },
    });
    const entry = {
      ...imageEntry(12, "/tmp/ditox/blob.png", 1536),
      favorite: true,
      source_app: "browser",
      hash: "abcdef1234567890",
    };
    const splitFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <PreviewPane config={config} entry={entry} rows={6} width={72} />
        </Shell>
      ),
      78,
      8,
    );

    expect(splitFrame).toContain("PIC|12|browser|32x24|abcdef");
    expect(splitFrame).toContain("1.5 KiB|/tmp/ditox/blob.png|saved");

    const fullSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <FullPreview config={config} entry={entry} rows={6} width={72} offset={0} onScroll={() => {}} />
        </Shell>
      ),
      78,
      8,
    );
    expect(findSpan(fullSpans, "PIC|12|browser|1.5 KiB|abcdef1234|saved")).toBeDefined();
  });

  test("renders extended row metadata template placeholders", async () => {
    const config = resolveTuiConfig({
      labels: {
        rowMetaTemplate: "{entryIdPrefix}{id}|{sourceApp}|{mime}|{dimensions}|{hash}|{pinnedSlot}",
        rowPinnedSlotTemplate: "{pinnedRaw}",
        rowPinnedLabel: "P",
        entryIdPrefix: "clip:",
      },
      layout: {
        rowMetaHashLength: 6,
        rowMetaPreviewGap: 1,
        rowPreviewReservedWidth: 56,
        rowPreviewMaxWidth: 16,
      },
    });
    const entry = {
      ...imageEntry(14, "/tmp/ditox/blob.png", 1536),
      favorite: true,
      source_app: "browser",
      hash: "abcdef1234567890",
    };

    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[entry]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={3}
            width={76}
            widthPercent={100}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      80,
      5,
    );

    expect(frame).toContain("clip:14|browser|image/png|32x24|abcdef|P");
    expect(frame).toContain("image/png 32x24");
  });

  test("bounds long custom row metadata before rendering the preview text", async () => {
    const config = resolveTuiConfig({
      labels: {
        rowMetaTemplate: "{entryIdPrefix}{id}|{sourceApp}|{mime}|{hashFull}",
        entryIdPrefix: "clip:",
        textTruncationMarker: "~",
      },
      layout: {
        rowMarkerGap: 1,
        rowMetaPreviewGap: 1,
        rowPreviewReservedWidth: 16,
        rowPreviewMaxWidth: 12,
      },
    });
    const entry = {
      ...textEntry(21, "preview text", false),
      source_app: "very-long-browser-process",
      hash: "abcdef1234567890",
    };

    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[entry]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={3}
            width={36}
            widthPercent={100}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      40,
      5,
    );

    expect(frame).toContain("clip:21|very~");
    expect(frame).toContain("preview text");
    expect(frame).not.toContain("very-long-browser-process");
  });

  test("renders configurable row spacing without overfilling the list viewport", async () => {
    const config = resolveTuiConfig({
      chrome: {
        panelBorder: false,
        normalMarker: "",
        selectedMarker: ">",
      },
      layout: {
        rowSpacing: 1,
        rowMarkerGap: 1,
        showRowMetadata: false,
        showScrollbar: false,
        panelPaddingX: 0,
      },
    });

    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[textEntry(1, "one", false), textEntry(2, "two", false), textEntry(3, "three", false), textEntry(4, "four", false)]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={5}
            width={24}
            widthPercent={100}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      28,
      5,
    );

    const lines = frame.split("\n");
    const firstRow = lines.findIndex((line) => line.includes("one"));
    expect(firstRow).toBeGreaterThanOrEqual(0);
    expect(lines[firstRow + 1]?.trim()).toBe("");
    expect(lines[firstRow + 2]).toContain("two");
    expect(frame).toContain("three");
    expect(frame).not.toContain("four");
  });

  test("aligns row content, metadata, and preview text independently", async () => {
    const config = resolveTuiConfig({
      labels: {
        rowMetaTemplate: "M",
      },
      chrome: {
        listBorder: false,
        selectedMarker: "",
        normalMarker: "",
      },
      layout: {
        listPaddingX: 0,
        showScrollbar: false,
        rowContentAlign: "right",
        rowMetadataAlign: "right",
        rowPreviewAlign: "center",
        rowMarkerGap: 0,
        rowMetaPreviewGap: 1,
        rowPreviewReservedWidth: 12,
        rowPreviewMaxWidth: 10,
      },
    });

    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[textEntry(81, "abc", false)]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={2}
            width={40}
            widthPercent={100}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      40,
      3,
    );

    expect(columnOf(frame, "M")).toBe(32);
    expect(columnOf(frame, "abc")).toBe(37);
  });

  test("renders configurable split and full preview line spacing", async () => {
    const config = resolveTuiConfig({
      chrome: {
        panelBorder: false,
      },
      layout: {
        showMetadata: false,
        showFullPreviewMetadata: false,
        showPreviewGutter: false,
        showFullPreviewGutter: false,
        previewLineSpacing: 1,
        fullPreviewLineSpacing: 1,
        fullPreviewScrollInsetRows: 0,
        panelPaddingX: 0,
      },
    });
    const entry = textEntry(41, "alpha\nbeta\ngamma\ndelta", false);

    const splitFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <PreviewPane config={config} entry={entry} rows={5} width={32} />
        </Shell>
      ),
      34,
      5,
    );
    const splitLines = splitFrame.split("\n");
    expect(splitLines[0]).toContain("alpha");
    expect(splitLines[1]?.trim()).toBe("");
    expect(splitLines[2]).toContain("beta");
    expect(splitLines[3]?.trim()).toBe("");
    expect(splitFrame).not.toContain("gamma");
    expect(splitFrame).not.toContain("delta");

    const fullFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <FullPreview config={config} entry={entry} rows={6} width={32} offset={0} onScroll={() => {}} />
        </Shell>
      ),
      34,
      6,
    );
    const fullLines = fullFrame.split("\n");
    expect(fullLines[0]).toContain("alpha");
    expect(fullLines[1]?.trim()).toBe("");
    expect(fullLines[2]).toContain("beta");
    expect(fullLines[3]?.trim()).toBe("");
    expect(fullFrame).toContain("gamma");
    expect(fullFrame).not.toContain("delta");
  });

  test("styles spacer surfaces independently", async () => {
    const config = resolveTuiConfig({
      chrome: {
        panelBorder: false,
      },
      styles: {
        rowSpacer: { bg: "#331111" },
        previewSpacer: { bg: "#113311" },
        fullPreviewSpacer: { bg: "#111133" },
      },
      layout: {
        rowSpacing: 1,
        previewLineSpacing: 1,
        fullPreviewLineSpacing: 1,
        showMetadata: false,
        showFullPreviewMetadata: false,
        showPreviewGutter: false,
        showFullPreviewGutter: false,
        showScrollbar: false,
        fullPreviewScrollInsetRows: 0,
        panelPaddingX: 0,
      },
    });
    const entry = textEntry(42, "alpha\nbeta\ngamma", false);

    const rowSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[textEntry(1, "one", false), textEntry(2, "two", false)]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={3}
            width={24}
            widthPercent={100}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      28,
      3,
    );
    expect(hasBackground(rowSpans, "#331111"), `row backgrounds: ${backgrounds(rowSpans).join(", ")}`).toBe(true);

    const previewSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <PreviewPane config={config} entry={entry} rows={5} width={32} />
        </Shell>
      ),
      34,
      5,
    );
    expect(hasBackground(previewSpans, "#113311"), `preview backgrounds: ${backgrounds(previewSpans).join(", ")}`).toBe(true);

    const fullPreviewSpans = await captureSpans(
      () => (
        <Shell config={config}>
          <FullPreview config={config} entry={entry} rows={6} width={32} offset={0} onScroll={() => {}} />
        </Shell>
      ),
      34,
      6,
    );
    expect(hasBackground(fullPreviewSpans, "#111133"), `full preview backgrounds: ${backgrounds(fullPreviewSpans).join(", ")}`).toBe(true);
  });

  test("can tune full preview text width independently from split preview", async () => {
    const entry = textEntry(33, "abcdefghijklmnopqrstuvwxyz", false);
    const config = resolveTuiConfig({
      labels: {
        fullPreviewGutterSeparator: ">",
      },
      layout: {
        showMetadata: false,
        showFullPreviewMetadata: false,
        previewTextWidthInset: 0,
        fullPreviewTextWidthInset: 24,
        previewGutterWidth: 1,
        fullPreviewGutterWidth: 1,
      },
    });

    const splitFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <PreviewPane config={config} entry={entry} rows={5} width={30} widthPercent={100} />
        </Shell>
      ),
      78,
      7,
    );
    expect(splitFrame).toContain("abcdefghijklmnopqrstuvwxyz");

    const fullFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <FullPreview config={config} entry={entry} rows={5} width={30} offset={0} onScroll={() => {}} />
        </Shell>
      ),
      78,
      7,
    );
    // Full preview now soft-wraps at the tuned text width (6 cols here) instead
    // of hard-truncating, so the whole string is visible across rows while the
    // per-row width is still capped (no 7-char run on a single line).
    expect(fullFrame).toContain("1>abcdef");
    expect(fullFrame).not.toContain("abcdefg");
  });

  test("can align split and full preview gutters independently", async () => {
    const entry = textEntry(93, "AB", false);
    const config = resolveTuiConfig({
      labels: {
        previewGutterSeparator: "|",
        fullPreviewGutterSeparator: ">",
      },
      chrome: {
        previewBorder: false,
        fullPreviewBorder: false,
      },
      layout: {
        showMetadata: false,
        showFullPreviewMetadata: false,
        previewPaddingX: 0,
        fullPreviewPaddingX: 0,
        previewGutterWidth: 4,
        fullPreviewGutterWidth: 4,
        previewGutterAlign: "left",
        fullPreviewGutterAlign: "center",
        fullPreviewScrollInsetRows: 0,
      },
    });

    const splitFrame = await captureFrame(
      () => <PreviewPane config={config} entry={entry} rows={2} width={16} widthPercent={100} />,
      16,
      2,
    );
    expect(lineContaining(splitFrame, "AB").startsWith("1   |AB")).toBe(true);

    const fullFrame = await captureFrame(
      () => <FullPreview config={config} entry={entry} rows={2} width={16} offset={0} onScroll={() => {}} />,
      16,
      2,
    );
    expect(lineContaining(fullFrame, "AB").startsWith(" 1  >AB")).toBe(true);
  });

  test("can hide split and full preview gutters for clean reader layouts", async () => {
    const config = resolveTuiConfig({
      labels: {
        previewGutterSeparator: " GG ",
        previewTypeGutter: "TYPEGUT",
        previewMimeGutter: "MIMEGUT",
      },
      layout: {
        showMetadata: false,
        showFullPreviewMetadata: false,
        showPreviewGutter: false,
        showFullPreviewGutter: false,
        imagePreviewMode: "metadata",
        panelPaddingX: 0,
      },
    });

    const splitFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <PreviewPane config={config} entry={imageEntry(5)} rows={6} width={48} />
        </Shell>
      ),
      54,
      8,
    );

    expect(splitFrame).toContain("Image entry");
    expect(splitFrame).toContain("image/png");
    expect(splitFrame).not.toContain("TYPEGUT");
    expect(splitFrame).not.toContain("MIMEGUT");
    expect(splitFrame).not.toContain(" GG ");

    const fullFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <FullPreview
            config={config}
            entry={textEntry(6, "gutterless full preview line", false)}
            rows={5}
            width={60}
            offset={0}
            onScroll={() => {}}
          />
        </Shell>
      ),
      66,
      7,
    );

    expect(fullFrame).toContain("gutterless full preview line");
    expect(fullFrame).not.toContain(" GG ");
  });

  test("renders customized clear and help overlays", async () => {
    const config = resolveTuiConfig({
      labels: {
        clearTitle: "wipe",
        clearPrefix: "Destroy",
        clearKindImages: "pictures",
        clearPromptTemplate: "{kind} <- {prefix}",
        clearPinnedUnsafeHint: "saved clips included",
        confirmHint: "commit/cancel",
        confirmHintTemplate: "choose: {hint}",
        helpTitle: "keys",
        helpPaste: "send selected clip",
        helpSearchCopyMatches: "yank matches",
        keyAlternativeSeparator: " or ",
        keyGroupSeparator: " + ",
      },
      keyBindings: {
        copyPaste: "ctrl+p,enter",
        searchCopyMatches: "ctrl+g",
      },
    });
    const clearState = {
      ...initialState(),
      mode: "confirm-clear" as const,
      clearKind: "images" as const,
      clearPreserveFavorites: false,
    };

    const clearFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={clearState} />
        </Shell>
      ),
      76,
      8,
    );
    expect(clearFrame).toContain("wipe");
    expect(clearFrame).toContain("pictures <- Destroy");
    expect(clearFrame).toContain("saved clips included");
    expect(clearFrame).toContain("choose: commit/cancel");

    const helpFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...initialState(), mode: "help" }} />
        </Shell>
      ),
      88,
      17,
    );
    expect(helpFrame).toContain("keys");
    expect(helpFrame).toContain("ctrl+p or enter");
    expect(helpFrame).toContain("send selected clip");
    expect(helpFrame).toContain("ctrl+g");
    expect(helpFrame).toContain("yank matches");
    expect(helpFrame).toContain("space");
    expect(helpFrame).toContain("x + s");
  });

  test("can reorder and hide help overlay rows from file config", async () => {
    const config = resolveTuiConfig({
      helpOrder: ["searchCopyMatches", "paste"],
      labels: {
        helpTitle: "mini keys",
        helpPaste: "send clip",
        helpSearchCopyMatches: "copy filtered",
      },
      keyBindings: {
        copyPaste: "enter",
        searchCopyMatches: "ctrl+g",
      },
    });
    const helpFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...initialState(), mode: "help" }} />
        </Shell>
      ),
      72,
      8,
    );

    expect(helpFrame).toContain("mini keys");
    expect(helpFrame.indexOf("copy filtered")).toBeLessThan(helpFrame.indexOf("send clip"));
    expect(helpFrame).toContain("ctrl+g");
    expect(helpFrame).toContain("enter");
    expect(helpFrame).not.toContain("move selection");
    expect(helpFrame).not.toContain("open preview");
  });

  test("can align help overlay key labels inside the key column", async () => {
    const config = resolveTuiConfig({
      chrome: {
        helpOverlayBorder: false,
      },
      layout: {
        helpOverlayHeight: 3,
        helpOverlayPaddingX: 0,
        helpKeyWidth: 6,
        helpKeyAlign: "right",
      },
      helpOrder: ["paste"],
      labels: {
        helpPaste: "paste now",
      },
      keyBindings: {
        copyPaste: "p",
      },
    });
    const helpFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...initialState(), mode: "help" }} />
        </Shell>
      ),
      24,
      4,
    );

    expect(lineContaining(helpFrame, "paste now")).toContain("     ppaste now");
  });

  test("can opt into extended help rows for hidden action groups", async () => {
    const config = resolveTuiConfig({
      layout: {
        helpOverlayHeight: 10,
        helpKeyWidth: 18,
      },
      helpOrder: ["quit", "previewNavigation", "previewBack", "output", "searchEdit", "confirmChoice"],
      labels: {
        helpTitle: "all keys",
        helpQuit: "leave picker",
        helpPreviewNavigation: "move preview",
        helpPreviewBack: "close preview",
        helpOutput: "print selection",
        helpSearchEdit: "edit search",
        helpConfirmChoice: "answer dialog",
        keyGroupSeparator: " + ",
      },
      keyBindings: {
        quit: "q",
        forceQuit: "ctrl+c",
        previewUp: "k",
        previewDown: "j",
        previewPageUp: "u",
        previewPageDown: "d",
        previewBack: "esc",
        output: "o",
        searchBackspace: "bs",
        searchApply: "ret",
        searchCancel: "esc",
        confirmYes: "yes",
        confirmNo: "no",
      },
    });
    const helpFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...initialState(), mode: "help" }} />
        </Shell>
      ),
      78,
      12,
    );

    expect(helpFrame).toContain("all keys");
    expect(helpFrame).toContain("q + ctrl+c");
    expect(helpFrame).toContain("leave picker");
    expect(helpFrame).toContain("k + j + u + d");
    expect(helpFrame).toContain("move preview");
    expect(helpFrame).toContain("esc");
    expect(helpFrame).toContain("close preview");
    expect(helpFrame).toContain("o");
    expect(helpFrame).toContain("print selection");
    expect(helpFrame).toContain("bs + ret + esc");
    expect(helpFrame).toContain("edit search");
    expect(helpFrame).toContain("yes + no");
    expect(helpFrame).toContain("answer dialog");
  });

  test("renders customized structural layout dimensions without overflowing", async () => {
    const config = resolveTuiConfig({
      labels: {
        brand: "GEOM",
        searchTitle: "lookup",
        searchPrompt: "q:",
        searchInputTemplate: "{prompt}{query}",
        statusPasteHint: "send",
        statusCopyHint: "clip",
        statusPreviewHint: "read",
        statusSearchHint: "find",
        statusHelpHint: "keys",
      },
      layout: {
        headerHeight: 4,
        statusHeight: 2,
        searchOverlayHeight: 4,
        minPaneWidth: 18,
        splitPaneGap: 2,
        splitPaneWidthInset: 0,
        previewTextWidthInset: 2,
        imagePreviewRowInset: 2,
        fullPreviewWidthInset: 0,
        fullPreviewScrollInsetRows: 1,
        statusSeparatorPadding: 0,
        frameTitlePadding: 0,
        panelPaddingX: 0,
      },
      chrome: {
        statusSeparator: "/",
      },
    });
    const state = {
      ...initialState(),
      entries: [textEntry(1, "geometry tuned clip", false)],
      selectedIndex: 0,
      query: "geo",
      mode: "search" as const,
      status: "ready",
      watcher: null,
    };

    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={state} selectedCount={0} />
          <box flexGrow={1} flexDirection="row">
            <EntryList
              config={config}
              entries={state.entries}
              selectedIndex={state.selectedIndex}
              selectedIds={state.selectedIds}
              rows={4}
              width={28}
              widthPercent={48}
              query={state.query}
              onSelectEntry={() => {}}
              onScroll={() => {}}
            />
            <box width={config.layout.splitPaneGap} backgroundColor={surface(config, "splitPaneGap").bg} />
            <PreviewPane config={config} entry={state.entries[state.selectedIndex]} rows={4} width={28} widthPercent={50} />
          </box>
          <StatusLine config={config} status={state.status} watcher={state.watcher} width={72} />
          <ModeOverlay config={config} state={state} />
        </Shell>
      ),
      72,
      16,
    );

    expectFrameWithin(frame, 72, 16);
    expect(frame).toContain("GEOM");
    expect(frame).toContain("╮  ╭");
    expect(frame).toContain("lookup");
    expect(frame).toContain("q:geo");
    expect(frame).toContain("enter send");
    expect(frame).toContain("ctrl+y clip");
    expect(frame).toContain("space / right read");
  });

  test("renders configurable vertical padding for header, status, and overlays", async () => {
    const config = resolveTuiConfig({
      labels: {
        brand: "VPAD",
        headerLineTemplate: "{brand}",
        statusLineTemplate: "{operation}",
        searchPrompt: "q:",
        searchInputTemplate: "{prompt}{query}",
      },
      chrome: {
        panelBorder: false,
        overlayBorder: false,
      },
      layout: {
        headerHeight: 3,
        statusHeight: 3,
        searchOverlayHeight: 3,
        headerPaddingX: 0,
        headerPaddingY: 1,
        statusPaddingX: 0,
        statusPaddingY: 1,
        overlayPaddingX: 0,
        overlayPaddingY: 1,
      },
    });
    const state = {
      ...initialState(),
      mode: "search" as const,
      query: "needle",
      status: "done",
    };

    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={state} selectedCount={0} />
          <StatusLine config={config} status={state.status} watcher={null} width={40} />
          <ModeOverlay config={config} state={state} />
        </Shell>
      ),
      40,
      9,
    );
    const lines = frame.split("\n");

    expect(lines[0]?.trim()).toBe("");
    expect(lines[1]).toContain("VPAD");
    expect(lines[3]?.trim()).toBe("");
    expect(lines[4]).toContain("done");
    expect(lines[6]?.trim()).toBe("");
    expect(lines[7]).toContain("q:needle");
  });

  test("renders borderless panel and overlay chrome when configured", async () => {
    const config = resolveTuiConfig({
      labels: {
        brand: "FLAT",
        historyTitle: "hidden-history-title",
        previewTitle: "hidden-preview-title",
        searchTitle: "hidden-search-title",
        searchPrompt: "find:",
      },
      chrome: {
        panelBorder: false,
        overlayBorder: false,
        normalMarker: ".",
      },
      layout: {
        listWidthPercent: 50,
        panelPaddingX: 0,
        overlayPaddingX: 0,
      },
    });
    const state = {
      ...initialState(),
      entries: [textEntry(1, "flat borderless content", false)],
      selectedIndex: 0,
      query: "flat",
      mode: "search" as const,
    };
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={state} selectedCount={0} />
          <box flexGrow={1} flexDirection="row">
            <EntryList
              config={config}
              entries={state.entries}
              selectedIndex={state.selectedIndex}
              selectedIds={state.selectedIds}
              rows={4}
              width={34}
              query={state.query}
              onSelectEntry={() => {}}
              onScroll={() => {}}
            />
            <PreviewPane config={config} entry={state.entries[state.selectedIndex]} rows={4} width={34} />
          </box>
          <ModeOverlay config={config} state={state} />
        </Shell>
      ),
      80,
      12,
    );

    expect(frame).toContain("FLAT");
    expect(frame).toContain("flat borderless content");
    expect(frame).toContain("find:flat");
    expect(frame).not.toContain("hidden-history-title");
    expect(frame).not.toContain("hidden-preview-title");
    expect(frame).not.toContain("hidden-search-title");
    expect(frame).not.toContain("╭");
    expect(frame).not.toContain("─");
  });

  test("renders independently configured panel and overlay chrome", async () => {
    const config = resolveTuiConfig({
      labels: {
        appTitle: "visible-header-title",
        historyTitle: "hidden-list-title",
        previewTitle: "visible-preview-title",
        previewModeTitle: "visible-full-title",
        previewBackHint: "visible-back",
        searchTitle: "visible-search-title",
        deleteTitle: "hidden-delete-title",
        searchInputTemplate: "{query}",
        deleteOneTemplate: "chrome-delete",
        confirmHintTemplate: "{hint}",
        confirmHint: "chrome-confirm",
      },
      chrome: {
        panelBorder: false,
        overlayBorder: false,
        headerBorder: true,
        listBorder: false,
        previewBorder: true,
        fullPreviewBorder: true,
        searchOverlayBorder: true,
        dangerOverlayBorder: false,
        showPanelTitles: false,
        showOverlayTitles: false,
        showHeaderTitle: true,
        showListTitle: true,
        showPreviewTitle: true,
        showFullPreviewTitle: true,
        showSearchOverlayTitle: true,
        showDangerOverlayTitle: true,
        headerBorderStyle: "single",
        previewBorderStyle: "double",
        fullPreviewBorderStyle: "heavy",
        searchOverlayBorderStyle: "rounded",
      },
      layout: {
        listWidthPercent: 50,
        panelPaddingX: 0,
        overlayPaddingX: 0,
      },
    });
    const entries = [textEntry(1, "chrome content", false)];
    const state = { ...initialState(), entries, selectedIndex: 0, query: "chrome-query", mode: "search" as const };
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={state} selectedCount={0} />
          <box flexGrow={1} flexDirection="row">
            <EntryList
              config={config}
              entries={entries}
              selectedIndex={0}
              selectedIds={new Set()}
              rows={4}
              width={34}
              query=""
              onSelectEntry={() => {}}
              onScroll={() => {}}
            />
            <PreviewPane config={config} entry={entries[0]} rows={4} width={34} />
          </box>
          <ModeOverlay config={config} state={state} />
        </Shell>
      ),
      90,
      14,
    );
    expect(frame).toContain("visible-header-title");
    expect(frame).toContain("visible-preview-title");
    expect(frame).toContain("visible-search-title");
    expect(frame).toContain("chrome content");
    expect(frame).not.toContain("hidden-list-title");

    const deleteFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...state, mode: "confirm-delete" as const }} />
        </Shell>
      ),
      72,
      6,
    );
    expect(deleteFrame).toContain("chrome-delete");
    expect(deleteFrame).not.toContain("hidden-delete-title");
    expect(deleteFrame).not.toContain("╭");

    const fullFrame = await captureFrame(
      () => <FullPreview config={config} entry={entries[0]} rows={5} width={72} offset={0} onScroll={() => {}} />,
      72,
      7,
    );
    expect(fullFrame).toContain("visible-full-title");
    expect(fullFrame).toContain("visible-back");
  });

  test("renders independently configured panel and overlay title alignment", async () => {
    const entry = textEntry(1, "title alignment content", false);
    const config = resolveTuiConfig({
      labels: {
        appTitle: "HEADER-TITLE",
        brand: "ALIGN",
        historyTitle: "LIST-TITLE",
        listPositionTemplate: "LIST-BOTTOM",
        previewTitle: "PREVIEW-TITLE",
        previewEntryTitleTemplate: "PREVIEW-BOTTOM",
        previewModeTitle: "FULL-TITLE",
        previewBackHint: "FULL-BACK",
        fullPreviewBottomTitleTemplate: "FULL-BOTTOM",
        searchTitle: "SEARCH-TITLE",
        searchInputTemplate: "{query}",
      },
      chrome: {
        panelBorderStyle: "single",
        overlayBorderStyle: "single",
        headerTitleAlignment: "right",
        listTitleAlignment: "center",
        previewTitleAlignment: "right",
        fullPreviewTitleAlignment: "left",
        listBottomTitleAlignment: "right",
        previewBottomTitleAlignment: "center",
        fullPreviewBottomTitleAlignment: "right",
        searchOverlayTitleAlignment: "center",
      },
      layout: {
        frameTitlePadding: 0,
        listWidthPercent: 100,
        searchOverlayHeight: 3,
      },
    });

    const headerFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={{ ...initialState(), entries: [entry], selectedIndex: 0 }} selectedCount={0} />
        </Shell>
      ),
      50,
      4,
    );
    expect(columnOf(headerFrame, "HEADER-TITLE")).toBeGreaterThan(25);

    const listFrame = await captureFrame(
      () => (
        <EntryList
          config={config}
          entries={[entry, textEntry(2, "second", false), textEntry(3, "third", false)]}
          selectedIndex={0}
          selectedIds={new Set()}
          rows={1}
          width={46}
          widthPercent={100}
          query=""
          onSelectEntry={() => {}}
          onScroll={() => {}}
        />
      ),
      50,
      4,
    );
    expect(columnOf(listFrame, "LIST-TITLE")).toBeGreaterThan(15);
    expect(columnOf(listFrame, "LIST-TITLE")).toBeLessThan(30);
    expect(columnOf(listFrame, "LIST-BOTTOM")).toBeGreaterThan(25);

    const previewFrame = await captureFrame(
      () => <PreviewPane config={config} entry={entry} rows={3} width={46} widthPercent={100} />,
      50,
      5,
    );
    expect(columnOf(previewFrame, "PREVIEW-TITLE")).toBeGreaterThan(25);
    expect(columnOf(previewFrame, "PREVIEW-BOTTOM")).toBeGreaterThan(12);
    expect(columnOf(previewFrame, "PREVIEW-BOTTOM")).toBeLessThan(30);

    const fullFrame = await captureFrame(
      () => <FullPreview config={config} entry={entry} rows={3} width={46} offset={0} onScroll={() => {}} />,
      50,
      5,
    );
    expect(columnOf(fullFrame, "FULL-TITLE")).toBeLessThan(8);
    expect(columnOf(fullFrame, "FULL-BOTTOM")).toBeGreaterThan(25);

    const searchFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...initialState(), query: "align", mode: "search" as const }} />
        </Shell>
      ),
      50,
      5,
    );
    expect(columnOf(searchFrame, "SEARCH-TITLE")).toBeGreaterThan(15);
    expect(columnOf(searchFrame, "SEARCH-TITLE")).toBeLessThan(30);
  });

  test("can pad title chrome asymmetrically", async () => {
    const config = resolveTuiConfig({
      labels: {
        historyTitle: "PAD",
      },
      chrome: {
        panelBorderStyle: "single",
      },
      layout: {
        frameTitlePadding: 0,
        frameTitlePaddingLeft: 2,
        frameTitlePaddingRight: 1,
        listWidthPercent: 100,
        showRowMetadata: false,
      },
    });
    const frame = await captureFrame(
      () => (
        <EntryList
          config={config}
          entries={[textEntry(1, "title padded content", false)]}
          selectedIndex={0}
          selectedIds={new Set()}
          rows={1}
          width={32}
          widthPercent={100}
          query=""
          onSelectEntry={() => {}}
          onScroll={() => {}}
        />
      ),
      36,
      4,
    );

    expect(frame).toContain("  PAD ─");
    expect(frame).not.toContain("  PAD  ─");
  });

  test("can hide title chrome while keeping panel and overlay borders", async () => {
    const config = resolveTuiConfig({
      labels: {
        appTitle: "hidden-app-title",
        brand: "TITLELESS",
        historyTitle: "hidden-history-title",
        previewTitle: "hidden-preview-title",
        searchTitle: "hidden-search-title",
        searchPrompt: "find:",
        listPositionTemplate: "hidden-position-{index}-{total}",
        previewEntryTitleTemplate: "hidden-entry-{id}",
        previewModeTitle: "hidden-full-title",
        previewBackHint: "hidden-back",
        fullPreviewBottomTitleTemplate: "hidden-full-bottom-{back}",
      },
      chrome: {
        showPanelTitles: false,
        showOverlayTitles: false,
        showListPositionTitle: false,
        showPreviewEntryTitle: false,
        showFullPreviewBottomTitle: false,
      },
      layout: {
        listWidthPercent: 50,
      },
    });
    const entries = [
      textEntry(1, "first visible titleless content", false),
      textEntry(2, "second visible titleless content", false),
      textEntry(3, "third visible titleless content", false),
    ];
    const state = {
      ...initialState(),
      entries,
      selectedIndex: 0,
      query: "titleless",
      mode: "search" as const,
    };
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={state} selectedCount={0} />
          <box flexGrow={1} flexDirection="row">
            <EntryList
              config={config}
              entries={entries}
              selectedIndex={0}
              selectedIds={new Set()}
              rows={1}
              width={34}
              query={state.query}
              onSelectEntry={() => {}}
              onScroll={() => {}}
            />
            <PreviewPane config={config} entry={entries[0]} rows={8} width={34} />
          </box>
          <ModeOverlay config={config} state={state} />
        </Shell>
      ),
      80,
      14,
    );

    expect(frame).toContain("╭");
    expect(frame).toContain("TITLELESS");
    expect(frame).toContain("first visible titleless");
    expect(frame).toContain("find:titleless");
    expect(frame).not.toContain("hidden-app-title");
    expect(frame).not.toContain("hidden-history-title");
    expect(frame).not.toContain("hidden-preview-title");
    expect(frame).not.toContain("hidden-search-title");
    expect(frame).not.toContain("hidden-position");
    expect(frame).not.toContain("hidden-entry");

    const fullFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <FullPreview
            config={config}
            entry={textEntry(9, "full titleless content", false)}
            rows={5}
            width={60}
            offset={0}
            onScroll={() => {}}
          />
        </Shell>
      ),
      66,
      8,
    );

    expect(fullFrame).toContain("╭");
    expect(fullFrame).toContain("full titleless content");
    expect(fullFrame).not.toContain("hidden-full-title");
    expect(fullFrame).not.toContain("hidden-full-bottom");
    expect(fullFrame).not.toContain("hidden-back");
  });

  test("supports a list-only browse layout without the split preview pane", async () => {
    const config = resolveTuiConfig({
      labels: {
        historyTitle: "clips",
        previewTitle: "hidden-preview",
      },
      layout: {
        showPreviewPane: false,
        rowPreviewReservedWidth: 12,
        rowPreviewMaxWidth: 80,
      },
    });
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <box flexGrow={1} flexDirection="row">
            <EntryList
              config={config}
              entries={[textEntry(1, "list only content fills the available width", false)]}
              selectedIndex={0}
              selectedIds={new Set()}
              rows={4}
              width={76}
              widthPercent={100}
              query=""
              onSelectEntry={() => {}}
              onScroll={() => {}}
            />
          </box>
        </Shell>
      ),
      82,
      7,
    );

    expect(frame).toContain("clips");
    expect(frame).toContain("list only content fills");
    expect(frame).not.toContain("hidden-preview");
  });

  test("can hide row metadata for a content-first list", async () => {
    const config = resolveTuiConfig({
      layout: {
        showRowMetadata: false,
        rowPreviewMaxWidth: 80,
      },
      chrome: {
        selectedMarker: ">",
      },
    });
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[textEntry(1, "content-first row keeps more text visible", false)]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={4}
            width={70}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      80,
      7,
    );

    expect(frame).toContain("> content-first row keeps more");
    expect(frame).not.toContain("TXT");
    expect(frame).not.toContain(" B ");
  });

  test("can render markerless rows with no reserved marker gap", async () => {
    const config = resolveTuiConfig({
      chrome: {
        panelBorder: false,
        selectedMarker: "",
        markedMarker: "",
        normalMarker: "",
      },
      layout: {
        showRowMetadata: false,
        rowMarkerGap: 0,
        rowPreviewMaxWidth: 80,
        panelPaddingX: 0,
      },
    });
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[textEntry(1, "markerless content begins at the row edge", false)]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={4}
            width={64}
            widthPercent={100}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      72,
      5,
    );

    expect(frame).toContain("markerless content begins at the row edge");
    expect(frame).not.toContain("> markerless");
    expect(frame).not.toContain("| markerless");
  });

  test("can reserve and align row marker slots", async () => {
    const config = resolveTuiConfig({
      chrome: {
        panelBorder: false,
        listBorder: false,
        selectedMarker: ">",
        normalMarker: ".",
      },
      layout: {
        showRowMetadata: false,
        rowMarkerWidth: 3,
        rowMarkerAlign: "right",
        rowMarkerGap: 1,
        rowPreviewMaxWidth: 80,
        panelPaddingX: 0,
      },
    });
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[textEntry(1, "marker slot selected", false), textEntry(2, "marker slot normal", false)]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={4}
            width={64}
            widthPercent={100}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      72,
      5,
    );

    expect(frame).toContain("  > marker slot selected");
    expect(frame).toContain("  . marker slot normal");
  });

  test("can render wider custom scrollbar glyphs", async () => {
    const config = resolveTuiConfig({
      chrome: {
        scrollbarThumb: "██",
        scrollbarTrack: "░░",
      },
      layout: {
        scrollbarWidth: 2,
        showRowMetadata: false,
        rowPreviewMaxWidth: 24,
      },
    });
    const entries = Array.from({ length: 8 }, (_, index) => textEntry(index + 1, `scroll row ${index + 1}`, false));
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={entries}
            selectedIndex={3}
            selectedIds={new Set()}
            rows={4}
            width={42}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      50,
      7,
    );

    expectFrameWithin(frame, 50, 7);
    expect(frame).toContain("██");
    expect(frame).toContain("░░");
  });

  test("can align narrow scrollbar glyphs inside a wider scrollbar", async () => {
    const config = resolveTuiConfig({
      chrome: {
        panelBorder: false,
        selectedMarker: "",
        normalMarker: "",
        scrollbarThumb: "#",
        scrollbarTrack: ".",
      },
      layout: {
        scrollbarWidth: 3,
        scrollbarAlign: "right",
        showRowMetadata: false,
        rowMarkerGap: 0,
        rowPreviewMaxWidth: 24,
        panelPaddingX: 0,
      },
    });
    const entries = Array.from({ length: 6 }, (_, index) => textEntry(index + 1, `right scroll ${index + 1}`, false));
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={entries}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={3}
            width={32}
            widthPercent={100}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      36,
      4,
    );

    expect(lineContaining(frame, "right scroll 1").endsWith("  #")).toBe(true);
    expect(lineContaining(frame, "right scroll 2").endsWith("  .")).toBe(true);
  });

  test("can place the list scrollbar on the left", async () => {
    const config = resolveTuiConfig({
      chrome: {
        panelBorder: false,
        selectedMarker: "",
        normalMarker: "",
        scrollbarThumb: "##",
        scrollbarTrack: "..",
      },
      layout: {
        scrollbarWidth: 2,
        scrollbarPlacement: "left",
        showRowMetadata: false,
        rowMarkerGap: 0,
        rowPreviewMaxWidth: 24,
        panelPaddingX: 0,
      },
    });
    const entries = Array.from({ length: 6 }, (_, index) => textEntry(index + 1, `left scroll ${index + 1}`, false));
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={entries}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={3}
            width={32}
            widthPercent={100}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      36,
      4,
    );

    expect(lineContaining(frame, "left scroll 1").startsWith("##left scroll 1")).toBe(true);
    expect(lineContaining(frame, "left scroll 2").startsWith("..left scroll 2")).toBe(true);
  });

  test("caps row preview text with maxEntryLength", async () => {
    const config = resolveTuiConfig({
      maxEntryLength: 10,
      layout: {
        imagePreviewMode: "metadata",
        listWidthPercent: 68,
      },
    });
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[textEntry(1, "abcdefghijklmnopqrstuvwxyz", false)]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={4}
            width={90}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      100,
      7,
    );

    expect(frame).toContain("abcdefg...");
    expect(frame).not.toContain("abcdefghijk");
  });

  test("renders empty states and image block previews", async () => {
    const config = resolveTuiConfig({
      labels: {
        noHistoryTitle: "nothing saved",
        noHistoryHelp: "copy first",
        noEntryTitle: "no clip",
        noEntryHelp: "choose a row",
        kindImage: "PIC",
        imagePreviewFallbackPrefix: "image",
        imagePreviewFallbackSeparator: " -> ",
        imagePreviewBlobMissing: "missing file",
      },
      layout: {
        imagePreviewMode: "blocks",
        imagePreviewMaxWidth: 4,
        imagePreviewMaxRows: 2,
        imagePreviewBlockGlyph: "█",
      },
    });
    const emptyFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <box flexGrow={1} flexDirection="row">
            <EntryList
              config={config}
              entries={[]}
              selectedIndex={0}
              selectedIds={new Set()}
              rows={5}
              width={34}
              query=""
              onSelectEntry={() => {}}
              onScroll={() => {}}
            />
            <PreviewPane config={config} entry={undefined} rows={5} width={40} />
          </box>
        </Shell>
      ),
      82,
      10,
    );
    expect(emptyFrame).toContain("nothing saved");
    expect(emptyFrame).toContain("copy first");
    expect(emptyFrame).toContain("no clip");
    expect(emptyFrame).toContain("choose a row");

    const fallbackFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <PreviewPane config={config} entry={imageEntry(5, null)} rows={12} width={48} />
        </Shell>
      ),
      58,
      14,
    );
    expect(fallbackFrame).toContain("image -> missing file");

    const dir = mkdtempSync(join(tmpdir(), "ditox-tui-render-"));
    try {
      const imagePath = join(dir, "preview.png");
      const bytes = rgbaPng(2, 2, [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 255, 255],
      ]);
      writeFileSync(imagePath, bytes);
      const frame = await captureFrame(
        () => (
          <Shell config={config}>
            <PreviewPane config={config} entry={imageEntry(4, imagePath, bytes.length)} rows={8} width={48} />
          </Shell>
        ),
        58,
        10,
      );
      expect(frame).toContain("PIC #4");
      expect(frame).toContain("█");
      expect(frame).not.toContain("▀");

      const openTuiConfig = resolveTuiConfig({
        layout: {
          imagePreviewMode: "blocks",
          imagePreviewRenderer: "opentui",
          imagePreviewMaxWidth: 2,
          imagePreviewMaxRows: 1,
          imagePreviewBlockGlyph: "█",
        },
      });
      const openTuiSpans = await captureSpans(
        () => (
          <Shell config={openTuiConfig}>
            <PreviewPane config={openTuiConfig} entry={imageEntry(8, imagePath, bytes.length)} rows={8} width={48} />
          </Shell>
        ),
        58,
        10,
      );
      const nativeCell = findSpan(openTuiSpans, "▙");
      expect(colorHex(nativeCell)).toBe("#ff0000");
      expect(colorHex(nativeCell, "bg")).toBe("#00ff00");

      const autoRendererConfig = resolveTuiConfig({
        layout: {
          imagePreviewMode: "blocks",
          imagePreviewRenderer: "auto",
          imagePreviewMaxWidth: 2,
          imagePreviewMaxRows: 1,
          imagePreviewBlockGlyph: "▀",
        },
      });
      const autoRendererFrame = await captureFrame(
        () => (
          <Shell config={autoRendererConfig}>
            <PreviewPane config={autoRendererConfig} entry={imageEntry(9, imagePath, bytes.length)} rows={8} width={48} />
          </Shell>
        ),
        58,
        10,
      );
      expect(autoRendererFrame).toContain("▀");
      expect(autoRendererFrame).not.toContain("▙");

      const nativeRequests: any[] = [];
      const nativeRendererFrame = await captureFrame(
        () => (
          <Shell config={autoRendererConfig}>
            <PreviewPane
              config={autoRendererConfig}
              entry={imageEntry(10, imagePath, bytes.length)}
              rows={8}
              width={48}
              imageTerminal={{
                columns: 58,
                rows: 10,
                resolution: { width: 580, height: 200 },
                capabilities: { kittyGraphics: true, sixel: false, nativeRenderer: true },
              }}
              imageManager={{ queue: (request: any) => nativeRequests.push(request), clear: () => {} }}
            />
          </Shell>
        ),
        58,
        10,
      );
      expect(nativeRendererFrame).not.toContain("▀");
      expect(nativeRequests).toHaveLength(1);
      expect(nativeRequests[0]).toMatchObject({
        protocol: "kitty",
        cols: 1,
        rows: 1,
        pixelWidth: 10,
        pixelHeight: 20,
        contentPixelWidth: 2,
        contentPixelHeight: 2,
        contentOffsetX: 4,
        contentOffsetY: 9,
      });

      const transparentConfig = resolveTuiConfig({
        layout: {
          imagePreviewMode: "blocks",
          imagePreviewRenderer: "text",
          imagePreviewMaxWidth: 2,
          imagePreviewMaxRows: 1,
          imagePreviewBackground: "#223344",
        },
      });
      const transparentPath = join(dir, "transparent.png");
      const transparentBytes = rgbaPng(1, 2, [
        [255, 0, 0, 255],
        [0, 0, 0, 0],
      ]);
      writeFileSync(transparentPath, transparentBytes);
      const transparentSpans = await captureSpans(
        () => (
          <Shell config={transparentConfig}>
            <PreviewPane config={transparentConfig} entry={imageEntry(7, transparentPath, transparentBytes.length)} rows={8} width={48} />
          </Shell>
        ),
        58,
        10,
      );
      const transparentCell = findSpan(transparentSpans, "▀");
      expect(colorHex(transparentCell)).toBe("#ff0000");
      expect(colorHex(transparentCell, "bg")).toBe("#223344");

      const webpPath = join(dir, "preview.webp");
      const webpBytes = webp2x2Red();
      writeFileSync(webpPath, webpBytes);
      const webpFrame = await captureFrameUntil(
        () => (
          <Shell config={config}>
            <PreviewPane config={config} entry={imageEntry(6, webpPath, webpBytes.length, "image/webp")} rows={8} width={48} />
          </Shell>
        ),
        58,
        10,
        (nextFrame) => nextFrame.includes("█"),
      );
      expect(webpFrame).toContain("PIC #6");
      expect(webpFrame).toContain("█");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("upgrades the first image from block fallback to native once resolution arrives", async () => {
    const dir = mkdtempSync(join(tmpdir(), "ditox-tui-resolution-"));
    try {
      const imagePath = join(dir, "first.png");
      const bytes = rgbaPng(2, 2, [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 255, 255],
      ]);
      writeFileSync(imagePath, bytes);

      const config = resolveTuiConfig({
        layout: {
          imagePreviewMode: "blocks",
          imagePreviewRenderer: "auto",
          imagePreviewMaxWidth: 2,
          imagePreviewMaxRows: 1,
          imagePreviewBlockGlyph: "▀",
        },
      });

      const nativeRequests: any[] = [];
      const [resolution, setResolution] = createSignal<{ width: number; height: number } | null>(null);
      const view = await testRender(
        () => (
          <Shell config={config}>
            <PreviewPane
              config={config}
              entry={imageEntry(11, imagePath, bytes.length)}
              rows={8}
              width={48}
              imageTerminal={{
                columns: 58,
                rows: 10,
                resolution: resolution(),
                capabilities: { kittyGraphics: true, sixel: false, nativeRenderer: true },
              }}
              imageManager={{ queue: (request: any) => nativeRequests.push(request), clear: () => {} }}
            />
          </Shell>
        ),
        { width: 58, height: 10 },
      );

      // First paint: the terminal has not reported its pixel resolution yet, so
      // the image must fall back to the low-res block renderer.
      await view.renderOnce();
      expect(view.captureCharFrame()).toContain("▀");
      expect(nativeRequests).toHaveLength(0);

      // Resolution arrives asynchronously: the same image upgrades to the native
      // renderer in place (no navigation required).
      setResolution({ width: 580, height: 200 });
      await view.renderOnce();
      expect(view.captureCharFrame()).not.toContain("▀");
      expect(nativeRequests.length).toBeGreaterThan(0);

      view.renderer.destroy();
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("renders configurable image protocol fallback notices", async () => {
    const dir = mkdtempSync(join(tmpdir(), "ditox-tui-protocol-notice-"));
    try {
      const imagePath = join(dir, "preview.png");
      const bytes = rgbaPng(2, 2, [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 255, 255],
      ]);
      writeFileSync(imagePath, bytes);
      const entry = imageEntry(17, imagePath, bytes.length);
      const config = resolveTuiConfig({
        labels: {
          imagePreviewProtocolUnsupported: "NO {protocol}",
          imagePreviewProtocolRendererUnavailable: "FALLBACK {protocol}",
          imagePreviewKittyProtocolName: "KITTY-GFX",
          imagePreviewSourceTemplate: "SRC {source}",
        },
        layout: {
          imagePreviewMode: "kitty",
          imagePreviewMaxWidth: 2,
          imagePreviewMaxRows: 1,
          imagePreviewNoticeVisibility: "protocol",
          showMetadata: false,
        },
        styles: {
          preview: { warning: "#aa5500" },
          fullPreview: { warning: "#bb6600" },
        },
        previewContentTones: {
          splitImageNotice: "warning",
          fullImageNotice: "warning",
        },
      });

      const splitSpans = await captureSpans(
        () => (
          <Shell config={config}>
            <PreviewPane config={config} entry={entry} rows={6} width={48} imageCapabilities={{ kittyGraphics: false, nativeRenderer: false }} />
          </Shell>
        ),
        58,
        8,
      );
      expect(colorHex(findSpan(splitSpans, "NO KITTY-GFX"))).toBe("#aa5500");

      const fullSpans = await captureSpans(
        () => (
          <Shell config={config}>
            <FullPreview
              config={config}
              entry={entry}
              rows={6}
              width={48}
              offset={0}
              imageCapabilities={{ kittyGraphics: true, nativeRenderer: false }}
              onScroll={() => {}}
            />
          </Shell>
        ),
        58,
        8,
      );
      expect(colorHex(findSpan(fullSpans, "FALLBACK KITTY-GFX"))).toBe("#bb6600");

      const sourceConfig = resolveTuiConfig({
        labels: {
          splitImagePreviewSourceTemplate: "SPLIT {source}",
          fullImagePreviewSourceTemplate: "FULL {source}",
          imagePreviewBlocksSource: "ansi image cells",
        },
        layout: {
          imagePreviewMode: "blocks",
          imagePreviewMaxWidth: 2,
          imagePreviewMaxRows: 1,
          imagePreviewNoticeVisibility: "always",
          showMetadata: false,
          showFullPreviewMetadata: false,
        },
      });
      const sourceFrame = await captureFrame(
        () => (
          <Shell config={sourceConfig}>
            <PreviewPane config={sourceConfig} entry={entry} rows={6} width={48} />
          </Shell>
        ),
        58,
        8,
      );
      expect(sourceFrame).toContain("SPLIT ansi image cells 2x1");

      const fullSourceFrame = await captureFrame(
        () => (
          <Shell config={sourceConfig}>
            <FullPreview config={sourceConfig} entry={entry} rows={6} width={48} offset={0} onScroll={() => {}} />
          </Shell>
        ),
        58,
        8,
      );
      expect(fullSourceFrame).toContain("FULL ansi image cells 2x1");

      const hiddenConfig = resolveTuiConfig({
        labels: { imagePreviewProtocolUnsupported: "NO {protocol}" },
        layout: {
          imagePreviewMode: "kitty",
          imagePreviewMaxWidth: 2,
          imagePreviewMaxRows: 1,
          imagePreviewNoticeVisibility: "never",
          showMetadata: false,
        },
      });
      const hiddenFrame = await captureFrame(
        () => (
          <Shell config={hiddenConfig}>
            <PreviewPane config={hiddenConfig} entry={entry} rows={6} width={48} imageCapabilities={{ kittyGraphics: false, nativeRenderer: false }} />
          </Shell>
        ),
        58,
        8,
      );
      expect(hiddenFrame).not.toContain("NO Kitty");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("can space split and full image notices from rendered previews", async () => {
    const dir = mkdtempSync(join(tmpdir(), "ditox-tui-image-notice-spacing-"));
    try {
      const imagePath = join(dir, "preview.png");
      const bytes = rgbaPng(2, 2, [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 255, 255],
      ]);
      writeFileSync(imagePath, bytes);
      const entry = imageEntry(33, imagePath, bytes.length);
      const config = resolveTuiConfig({
        labels: {
          splitImagePreviewSourceTemplate: "SPLIT-SOURCE {source}",
          fullImagePreviewSourceTemplate: "FULL-SOURCE {source}",
        },
        layout: {
          imagePreviewMode: "blocks",
          fullPreviewImageMode: "blocks",
          imagePreviewRenderer: "text",
          fullPreviewImageRenderer: "text",
          imagePreviewMaxWidth: 2,
          fullPreviewImageMaxWidth: 2,
          imagePreviewMaxRows: 1,
          fullPreviewImageMaxRows: 1,
          imagePreviewNoticeVisibility: "always",
          fullPreviewImageNoticeVisibility: "always",
          imagePreviewNoticeSpacing: 1,
          fullPreviewImageNoticeSpacing: 2,
          showMetadata: false,
          showFullPreviewMetadata: false,
        },
      });
      const splitFrame = await captureFrame(
        () => (
          <Shell config={config}>
            <PreviewPane config={config} entry={entry} rows={6} width={48} />
          </Shell>
        ),
        58,
        8,
      );
      expect(lineIndex(splitFrame, "SPLIT-SOURCE") - lineIndex(splitFrame, "▀")).toBe(2);

      const fullFrame = await captureFrame(
        () => (
          <Shell config={config}>
            <FullPreview config={config} entry={entry} rows={7} width={48} offset={0} onScroll={() => {}} />
          </Shell>
        ),
        58,
        9,
      );
      expect(lineIndex(fullFrame, "FULL-SOURCE") - lineIndex(fullFrame, "▀")).toBe(3);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("renders split and full image fallback copy independently", async () => {
    const config = resolveTuiConfig({
      labels: {
        splitImagePreviewFallbackPrefix: "SPLITIMG",
        splitImagePreviewFallbackSeparator: "<",
        fullImagePreviewFallbackPrefix: "FULLIMG",
        fullImagePreviewFallbackSeparator: ">",
        imagePreviewBlobMissing: "gone",
      },
      layout: {
        imagePreviewMode: "blocks",
        fullPreviewImageMode: "blocks",
        showMetadata: false,
        showFullPreviewMetadata: false,
      },
    });
    const entry = imageEntry(18, null);

    const splitFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <PreviewPane config={config} entry={entry} rows={5} width={48} />
        </Shell>
      ),
      58,
      7,
    );
    expect(splitFrame).toContain("SPLITIMG<gone");

    const fullFrame = await captureFrame(
      () => (
        <Shell config={config}>
          <FullPreview config={config} entry={entry} rows={5} width={48} offset={0} onScroll={() => {}} />
        </Shell>
      ),
      58,
      7,
    );
    expect(fullFrame).toContain("FULLIMG>gone");
  });

  test("reserves rendered image rows before windowing preview metadata", async () => {
    const dir = mkdtempSync(join(tmpdir(), "ditox-tui-image-row-budget-"));
    try {
      const imagePath = join(dir, "tall.png");
      const pixels: Array<[number, number, number, number]> = Array.from({ length: 16 }, (_, index) =>
        index % 2 === 0 ? [255, 0, 0, 255] : [0, 0, 255, 255],
      );
      const bytes = rgbaPng(2, 8, pixels);
      writeFileSync(imagePath, bytes);
      const entry = imageEntry(23, imagePath, bytes.length);
      const config = resolveTuiConfig({
        labels: {
          previewMetaHeaderTemplate: "IMGHEADER",
          previewMetaDetailsTemplate: "IMGDETAIL",
          fullPreviewMetaTemplate: "FULLHEADER",
          fullPreviewBottomTitleTemplate: "range {start}-{end}/{total}",
          imagePreviewSourceTemplate: "SRC {source}",
        },
        chrome: {
          showPreviewEntryTitle: false,
        },
        layout: {
          imagePreviewMode: "blocks",
          imagePreviewRenderer: "text",
          imagePreviewMaxWidth: 2,
          imagePreviewMaxRows: 4,
          imagePreviewRowInset: 0,
          imagePreviewNoticeVisibility: "always",
          previewMetaHeight: 3,
          fullPreviewMetaHeight: 1,
          fullPreviewScrollInsetRows: 0,
          showFullPreviewGutter: false,
        },
      });

      const splitFrame = await captureFrame(
        () => (
          <Shell config={config}>
            <PreviewPane config={config} entry={entry} rows={8} width={48} />
          </Shell>
        ),
        58,
        10,
      );
      expect(splitFrame).toContain("IMGHEADER");
      expect(splitFrame).toContain("SRC image blocks 2x4");
      expect(splitFrame).not.toContain("image/png");

      const fullFrame = await captureFrame(
        () => (
          <Shell config={config}>
            <FullPreview config={config} entry={entry} rows={8} width={48} offset={0} onScroll={() => {}} />
          </Shell>
        ),
        58,
        10,
      );
      expect(fullFrame).toContain("FULLHEADER");
      expect(fullFrame).toContain("range 1-2/6");
      expect(fullFrame).toContain("Image entry");
      expect(fullFrame).toContain("image/png");
      expect(fullFrame).not.toContain("blob");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("can tune full preview image row inset independently from split preview", async () => {
    const dir = mkdtempSync(join(tmpdir(), "ditox-tui-full-image-row-inset-"));
    try {
      const imagePath = join(dir, "tall.png");
      const pixels: Array<[number, number, number, number]> = Array.from({ length: 16 }, (_, index) =>
        index % 2 === 0 ? [255, 0, 0, 255] : [0, 0, 255, 255],
      );
      const bytes = rgbaPng(2, 8, pixels);
      writeFileSync(imagePath, bytes);
      const entry = imageEntry(24, imagePath, bytes.length);
      const config = resolveTuiConfig({
        labels: {
          imagePreviewSourceTemplate: "SRC {source}",
        },
        layout: {
          imagePreviewMode: "blocks",
          imagePreviewRenderer: "text",
          imagePreviewMaxWidth: 2,
          imagePreviewMaxRows: 4,
          imagePreviewRowInset: 6,
          fullPreviewImageRowInset: 0,
          imagePreviewNoticeVisibility: "always",
          showMetadata: false,
          showFullPreviewMetadata: false,
          showFullPreviewGutter: false,
        },
      });

      const splitFrame = await captureFrame(
        () => (
          <Shell config={config}>
            <PreviewPane config={config} entry={entry} rows={8} width={48} />
          </Shell>
        ),
        58,
        10,
      );
      expect(splitFrame).toContain("SRC image blocks 1x2");

      const fullFrame = await captureFrame(
        () => (
          <Shell config={config}>
            <FullPreview config={config} entry={entry} rows={8} width={48} offset={0} onScroll={() => {}} />
          </Shell>
        ),
        58,
        10,
      );
      expect(fullFrame).toContain("SRC image blocks 2x4");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("can tune full preview image max size independently from split preview", async () => {
    const dir = mkdtempSync(join(tmpdir(), "ditox-tui-full-image-max-size-"));
    try {
      const imagePath = join(dir, "square.png");
      const pixels: Array<[number, number, number, number]> = Array.from({ length: 32 * 32 }, (_, index) =>
        index % 2 === 0 ? [255, 0, 0, 255] : [0, 0, 255, 255],
      );
      const bytes = rgbaPng(32, 32, pixels);
      writeFileSync(imagePath, bytes);
      const entry = imageEntry(25, imagePath, bytes.length);
      const config = resolveTuiConfig({
        labels: {
          imagePreviewSourceTemplate: "SRC {source}",
        },
        layout: {
          imagePreviewMode: "blocks",
          imagePreviewRenderer: "text",
          imagePreviewMaxWidth: 8,
          imagePreviewMaxRows: 20,
          fullPreviewImageMaxWidth: 16,
          fullPreviewImageMaxRows: 20,
          imagePreviewRowInset: 0,
          fullPreviewImageRowInset: 0,
          imagePreviewNoticeVisibility: "always",
          showMetadata: false,
          showFullPreviewMetadata: false,
          showFullPreviewGutter: false,
        },
      });

      const splitFrame = await captureFrame(
        () => (
          <Shell config={config}>
            <PreviewPane config={config} entry={entry} rows={20} width={58} />
          </Shell>
        ),
        68,
        22,
      );
      expect(splitFrame).toContain("SRC image blocks 8x4");

      const fullFrame = await captureFrame(
        () => (
          <Shell config={config}>
            <FullPreview config={config} entry={entry} rows={20} width={58} offset={0} onScroll={() => {}} />
          </Shell>
        ),
        68,
        22,
      );
      expect(fullFrame).toContain("SRC image blocks 16x8");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("can tune full preview image rendering independently from split preview", async () => {
    const dir = mkdtempSync(join(tmpdir(), "ditox-tui-full-image-rendering-"));
    try {
      const imagePath = join(dir, "transparent.png");
      const bytes = rgbaPng(1, 2, [
        [255, 0, 0, 255],
        [0, 0, 0, 0],
      ]);
      writeFileSync(imagePath, bytes);
      const entry = imageEntry(26, imagePath, bytes.length);
      const config = resolveTuiConfig({
        labels: {
          imagePreviewSourceTemplate: "SRC {source}",
        },
        layout: {
          imagePreviewMode: "metadata",
          fullPreviewImageMode: "blocks",
          imagePreviewRenderer: "opentui",
          fullPreviewImageRenderer: "text",
          imagePreviewBlockGlyph: "█",
          fullPreviewImageBlockGlyph: "▓",
          imagePreviewBackground: "#112233",
          fullPreviewImageBackground: "#445566",
          imagePreviewNoticeVisibility: "never",
          fullPreviewImageNoticeVisibility: "always",
          showMetadata: false,
          showFullPreviewMetadata: false,
          showFullPreviewGutter: false,
        },
      });

      const splitFrame = await captureFrame(
        () => (
          <Shell config={config}>
            <PreviewPane config={config} entry={entry} rows={8} width={48} />
          </Shell>
        ),
        58,
        10,
      );
      expect(splitFrame).not.toContain("SRC image blocks");
      expect(splitFrame).not.toContain("▓");

      const fullFrame = await captureFrame(
        () => (
          <Shell config={config}>
            <FullPreview config={config} entry={entry} rows={8} width={48} offset={0} onScroll={() => {}} />
          </Shell>
        ),
        58,
        10,
      );
      expect(fullFrame).toContain("SRC image blocks 1x1");

      const fullSpans = await captureSpans(
        () => (
          <Shell config={config}>
            <FullPreview config={config} entry={entry} rows={8} width={48} offset={0} onScroll={() => {}} />
          </Shell>
        ),
        58,
        10,
      );
      const imageCell = findSpan(fullSpans, "▓");
      expect(colorHex(imageCell)).toBe("#ff0000");
      expect(colorHex(imageCell, "bg")).toBe("#445566");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("renders split and full image previews with configurable alignment", async () => {
    const dir = mkdtempSync(join(tmpdir(), "ditox-tui-image-align-"));
    try {
      const imagePath = join(dir, "align.png");
      const bytes = rgbaPng(2, 2, [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
      ]);
      writeFileSync(imagePath, bytes);
      const entry = imageEntry(27, imagePath, bytes.length);
      const config = resolveTuiConfig({
        chrome: {
          previewBorder: false,
          fullPreviewBorder: false,
        },
        layout: {
          imagePreviewMode: "blocks",
          fullPreviewImageMode: "blocks",
          imagePreviewRenderer: "text",
          fullPreviewImageRenderer: "text",
          imagePreviewBlockGlyph: "█",
          fullPreviewImageBlockGlyph: "▓",
          imagePreviewMaxWidth: 2,
          imagePreviewMaxRows: 1,
          fullPreviewImageMaxWidth: 2,
          fullPreviewImageMaxRows: 1,
          imagePreviewAlign: "center",
          fullPreviewImageAlign: "right",
          showMetadata: false,
          showFullPreviewMetadata: false,
          showPreviewGutter: false,
          showFullPreviewGutter: false,
        },
      });

      const splitFrame = await captureFrame(
        () => <PreviewPane config={config} entry={entry} rows={3} width={10} widthPercent={100} />,
        12,
        4,
      );
      expect(columnOf(splitFrame, "█")).toBe(4);

      const fullFrame = await captureFrame(
        () => <FullPreview config={config} entry={entry} rows={3} width={10} offset={0} onScroll={() => {}} />,
        12,
        4,
      );
      expect(columnOf(fullFrame, "▓")).toBe(7);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("can hide empty-state helper copy", async () => {
    const config = resolveTuiConfig({
      labels: {
        noMatchesTitle: "no filtered clips",
        noMatchesHelp: "hidden widen query",
      },
      layout: {
        showEmptyStateHelp: false,
      },
    });
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={5}
            width={44}
            query="needle"
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      54,
      8,
    );

    expect(frame).toContain("no filtered clips");
    expect(frame).not.toContain("hidden widen query");
  });

  test("aligns empty-state title and helper copy independently", async () => {
    const config = resolveTuiConfig({
      labels: {
        noMatchesTitle: "EMPTY",
        noMatchesHelp: "HELP",
      },
      chrome: {
        listBorder: false,
      },
      layout: {
        emptyStatePaddingX: 0,
        emptyStatePaddingY: 0,
        emptyStateTitleAlign: "center",
        emptyStateHelpAlign: "right",
        showScrollbar: false,
      },
    });
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={3}
            width={30}
            widthPercent={100}
            query="needle"
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      30,
      4,
    );

    expect(columnOf(frame, "EMPTY")).toBe(12);
    expect(columnOf(frame, "HELP")).toBe(25);
  });

  test("can space empty-state title and helper copy", async () => {
    const config = resolveTuiConfig({
      labels: {
        noHistoryTitle: "EMPTY-TITLE",
        noHistoryHelp: "EMPTY-HELP",
      },
      chrome: {
        listBorder: false,
      },
      layout: {
        emptyStatePaddingX: 0,
        emptyStatePaddingY: 0,
        emptyStateLineSpacing: 1,
        showScrollbar: false,
      },
    });
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={5}
            width={32}
            widthPercent={100}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      32,
      6,
    );

    expect(lineIndex(frame, "EMPTY-HELP") - lineIndex(frame, "EMPTY-TITLE")).toBe(2);
  });

  test("can vertically align empty-state copy", async () => {
    const config = resolveTuiConfig({
      labels: {
        noHistoryTitle: "LOW",
        noHistoryHelp: "hidden",
      },
      chrome: {
        listBorder: false,
      },
      layout: {
        emptyStatePaddingX: 0,
        emptyStatePaddingY: 0,
        emptyStateVerticalAlign: "bottom",
        showEmptyStateHelp: false,
        showScrollbar: false,
      },
    });
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <EntryList
            config={config}
            entries={[]}
            selectedIndex={0}
            selectedIds={new Set()}
            rows={5}
            width={20}
            widthPercent={100}
            query=""
            onSelectEntry={() => {}}
            onScroll={() => {}}
          />
        </Shell>
      ),
      20,
      6,
    );

    expect(lineIndex(frame, "LOW")).toBeGreaterThanOrEqual(4);
    expect(frame).not.toContain("hidden");
  });

  test("can choose image metadata preview fields", async () => {
    const config = resolveTuiConfig({
      labels: {
        previewTypeGutter: "type-hidden",
        previewHashGutter: "hash-hidden",
      },
      layout: {
        imagePreviewMode: "metadata",
        showMetadata: false,
        previewImageFields: ["mime", "size"],
      },
    });
    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <PreviewPane config={config} entry={imageEntry(4, null, 1536)} rows={8} width={48} />
        </Shell>
      ),
      58,
      10,
    );

    expect(frame).toContain("image/png");
    expect(frame).toContain("1.5 KiB");
    expect(frame).not.toContain("type-hidden");
    expect(frame).not.toContain("hash-hidden");
    expect(frame).not.toContain("Image entry");
  });

  test("keeps compact light theme frames inside narrow terminal bounds", async () => {
    const config = resolveTuiConfig({
      theme: "ditoxLight",
      labels: {
        brand: "LT",
        historyTitle: "h",
        previewTitle: "p",
        filterAll: "EVERYTHING",
        queryEmpty: "none",
        statusPasteHint: "paste",
        statusCopyHint: "copy",
        statusPreviewHint: "view",
        statusSearchHint: "find",
        statusHelpHint: "help",
      },
      chrome: {
        panelBorderStyle: "single",
        overlayBorderStyle: "single",
        selectedMarker: ">",
        markedMarker: "*",
        normalMarker: ".",
        statusSeparator: "/",
      },
      layout: {
        compactMode: true,
        listWidthPercent: 50,
        showMetadata: false,
        imagePreviewMode: "metadata",
        maxPreviewLines: 4,
      },
    });
    const entries = [
      textEntry(1, "compact content that should stay clipped inside the list pane", false),
      imageEntry(2),
    ];
    const state = {
      ...initialState(),
      entries,
      selectedIndex: 0,
      status: "ready",
      watcher: null,
    };

    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={state} selectedCount={state.selectedIds.size} />
          <box flexGrow={1} flexDirection="row">
            <EntryList
              config={config}
              entries={entries}
              selectedIndex={state.selectedIndex}
              selectedIds={state.selectedIds}
              rows={5}
              width={24}
              query={state.query}
              onSelectEntry={() => {}}
              onScroll={() => {}}
            />
            <PreviewPane config={config} entry={entries[state.selectedIndex]} rows={5} width={24} />
          </box>
          <StatusLine config={config} status={state.status} watcher={state.watcher} width={56} />
        </Shell>
      ),
      56,
      12,
    );

    expectFrameWithin(frame, 56, 12);
    expect(frame).toContain("LT");
    expect(frame).toContain("EVERYTHING");
    expect(frame).toContain("none");
    expect(frame).toContain("h");
    expect(frame).toContain("p");
    expect(frame).toContain("> TXT");
    expect(frame).toContain("compact");
    expect(frame).toContain("watcher stopped");
  });

  test("renders a viewport and input-state matrix without overflowing terminal bounds", async () => {
    const matrixConfig = resolveTuiConfig({
      labels: {
        brand: "MATRIX",
        pinnedViewTitle: "SAVED",
        selectedPrefix: "marked",
        selectedCountTemplate: "{count} {prefix}",
        headerSectionSeparator: "::",
        headerLabelSeparator: "=",
        previewMetaHashLabel: "digest",
        previewMetaSeparator: " ~ ",
        previewMetaLabelSeparator: ":",
        previewMetaHeaderTemplate: "{kind}/{id}/{hashLabel}:{hash}",
        previewMetaDetailsTemplate: "{size} @ {mime}{pinnedSuffix}",
        sizeKibUnit: "KB",
        ageSecondsUnit: "sec",
      },
      chrome: {
        selectedMarker: ">",
        selectedMarkedMarker: "*",
        markedMarker: "+",
        normalMarker: ".",
        statusSeparator: "/",
      },
      layout: {
        listWidthPercent: 42,
        previewMetaHashLength: 8,
        statusSeparatorPadding: 0,
        frameTitlePadding: 0,
        headerPaddingX: 0,
        statusPaddingX: 0,
        previewMetaPaddingX: 0,
        rowMarkerGap: 0,
        rowMetaPreviewGap: 1,
        rowPreviewReservedWidth: 18,
        imagePreviewMode: "metadata",
        showMetadata: true,
      },
      styles: {
        header: { bg: "#101820", fg: "#f3f7f8", accent: "#f2aa4c" },
        status: { bg: "#101820", fg: "#b9c8cf", accent: "#f2aa4c" },
        selectedRow: { bg: "#23343d", fg: "#ffffff", accent: "#f2aa4c" },
      },
    });
    const matrixEntries = [
      textEntry(1, "saved matrix clip with enough text for truncation", true),
      imageEntry(2, null, 2048),
      textEntry(3, "plain matrix clip", false),
    ];
    const matrixState = {
      ...initialState(),
      entries: matrixEntries,
      selectedIndex: 1,
      selectedIds: new Set([1, 2]),
      pinnedOnly: true,
      status: "stored 2",
      watcher: {
        running: false,
        paused: true,
        backend: "wayland",
        poll_interval_ms: 750,
        last_seen_ms: Date.now(),
        last_error: null,
      },
    };

    const compactConfig = resolveTuiConfig({
      theme: "ditoxLight",
      labels: {
        brand: "MIN",
        noMatchesTitle: "no filtered clips",
        noMatchesHelp: "widen query",
        noEntryTitle: "empty side",
        noEntryHelp: "pick history",
      },
      layout: {
        compactMode: true,
        listWidthPercent: 58,
        showScrollbar: false,
        showMetadata: false,
        showEmptyStateHelp: true,
        panelPaddingX: 0,
        statusSeparatorPadding: 1,
      },
      chrome: {
        panelBorderStyle: "single",
        normalMarker: ".",
        statusSeparator: ":",
      },
    });
    const compactState = {
      ...initialState(),
      query: "needle",
      status: "idle",
      watcher: {
        running: false,
        paused: false,
        backend: "wayland",
        poll_interval_ms: 750,
        last_seen_ms: Date.now() - 60_000,
        last_error: null,
      },
    };

    const previewConfig = resolveTuiConfig({
      labels: {
        previewModeTitle: "reader",
        previewBackHint: "leave",
        previewGutterSeparator: ":",
        kindText: "STR",
      },
      layout: {
        previewLineNumberWidth: 2,
        previewGutterWidth: 2,
        maxFullPreviewLines: 6,
        panelPaddingX: 0,
      },
    });

    const cases: Array<{ name: string; width: number; height: number; node: () => any; contains: string[] }> = [
      {
        name: "wide pinned state",
        width: 118,
        height: 18,
        node: () => (
          <Shell config={matrixConfig}>
            <HeaderBar config={matrixConfig} state={matrixState} selectedCount={matrixState.selectedIds.size} />
            <box flexGrow={1} flexDirection="row">
              <EntryList
                config={matrixConfig}
                entries={matrixEntries}
                selectedIndex={matrixState.selectedIndex}
                selectedIds={matrixState.selectedIds}
                rows={8}
                width={42}
                query={matrixState.query}
                onSelectEntry={() => {}}
                onScroll={() => {}}
              />
              <PreviewPane config={matrixConfig} entry={matrixEntries[matrixState.selectedIndex]} rows={8} width={64} />
            </box>
            <StatusLine config={matrixConfig} status={matrixState.status} watcher={matrixState.watcher} width={118} />
          </Shell>
        ),
        contains: ["MATRIX::filter=SAVED", "::query=-", "::mode=2 marked", "*IMG", "IMG/2/digest:image-2", "2.0 KB @ image/png", "watcher paused", "/"],
      },
      {
        name: "narrow empty search",
        width: 48,
        height: 10,
        node: () => (
          <Shell config={compactConfig}>
            <HeaderBar config={compactConfig} state={compactState} selectedCount={0} />
            <box flexGrow={1} flexDirection="row">
              <EntryList
                config={compactConfig}
                entries={[]}
                selectedIndex={0}
                selectedIds={new Set()}
                rows={4}
                width={22}
                query={compactState.query}
                onSelectEntry={() => {}}
                onScroll={() => {}}
              />
              <PreviewPane config={compactConfig} entry={undefined} rows={4} width={20} />
            </box>
            <StatusLine config={compactConfig} status={compactState.status} watcher={compactState.watcher} width={48} />
          </Shell>
        ),
        contains: ["MIN", "needle", "no filtered clips", "widen query", "empty side", "watcher stale"],
      },
      {
        name: "medium full preview",
        width: 52,
        height: 9,
        node: () => (
          <Shell config={previewConfig}>
            <FullPreview
              config={previewConfig}
              entry={textEntry(8, "alpha\nbeta\ngamma\ndelta\nepsilon", false)}
              rows={7}
              width={48}
              offset={2}
              onScroll={() => {}}
            />
          </Shell>
        ),
        contains: ["reader", "STR #8", "3:gamma", "4:delta", "leave"],
      },
    ];

    for (const item of cases) {
      const frame = await captureFrame(item.node, item.width, item.height);
      expectFrameWithin(frame, item.width, item.height);
      for (const expected of item.contains) {
        expect(frame, item.name).toContain(expected);
      }
    }
  });

  test("full list and long soft-wrapped preview text keep the status line and pane borders on screen", async () => {
    // Regression: each pane's `rows` budget includes its own border/padding
    // chrome. A full history list or a long wrapped preview used to render
    // `rows` content rows PLUS chrome, growing past the pane slot and pushing
    // the bottom borders and status line off the terminal.
    const config = resolveTuiConfig({});
    const longText = Array.from({ length: 30 }, (_, index) => `line ${index} ${"persistently-long-token ".repeat(8)}`).join("\n");
    const entries = [textEntry(1, longText, false), ...Array.from({ length: 40 }, (_, index) => textEntry(index + 2, `short clip ${index}`, false))];
    const state = { ...initialState(), entries, selectedIndex: 0, status: "ready", watcher: null };

    const width = 100;
    const height = 30;
    const headerRows = config.layout.showHeader ? config.layout.headerHeight : 0;
    const statusRows = config.layout.showStatusLine ? config.layout.statusHeight : 0;
    const listRows = Math.max(1, height - headerRows - statusRows);

    const frame = await captureFrame(
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={state} selectedCount={0} width={width} />
          <box flexDirection="row" flexGrow={1}>
            <EntryList
              config={config}
              entries={entries}
              selectedIndex={0}
              selectedIds={new Set<number>()}
              rows={listRows}
              width={40}
              query=""
              onSelectEntry={() => {}}
              onScroll={() => {}}
            />
            <PreviewPane config={config} entry={entries[0]} rows={listRows} width={58} />
          </box>
          <StatusLine config={config} status={state.status} watcher={state.watcher} width={width} mode="browse" />
        </Shell>
      ),
      width,
      height,
    );

    expectFrameWithin(frame, width, height);
    const lines = frame.replace(/\n$/, "").split("\n");
    // Status line is the last laid-out row, with both pane bottom borders
    // right above it.
    expect(lines[height - 1]).toContain("ready");
    expect(lines[height - 2]).toContain("╰");
    expect(lines[height - 2]).toContain("╯");
  });
});

async function captureFrame(node: () => any, width: number, height: number): Promise<string> {
  const view = await testRender(node, { width, height });
  try {
    await view.renderOnce();
    return view.captureCharFrame();
  } finally {
    view.renderer.destroy();
  }
}

async function captureSpans(node: () => any, width: number, height: number): Promise<any[]> {
  const view = await testRender(node, { width, height });
  try {
    await view.renderOnce();
    return view.captureSpans().lines.flatMap((line: any) => line.spans ?? []);
  } finally {
    view.renderer.destroy();
  }
}

async function captureFrameUntil(node: () => any, width: number, height: number, done: (frame: string) => boolean): Promise<string> {
  const view = await testRender(node, { width, height });
  try {
    let frame = "";
    for (let attempt = 0; attempt < 20; attempt += 1) {
      await view.renderOnce();
      frame = view.captureCharFrame();
      if (done(frame)) return frame;
      await new Promise((resolve) => setTimeout(resolve, 10));
    }
    return frame;
  } finally {
    view.renderer.destroy();
  }
}

function findSpan(spans: any[], text: string): any {
  const span = spans.find((candidate) => String(candidate.text).includes(text));
  expect(span, `span containing ${text}`).toBeDefined();
  return span;
}

function colorHex(span: any, field: "fg" | "bg" = "fg"): string {
  const raw = span[field]?.buffer ?? span[field];
  const bytes =
    raw instanceof Uint8Array
      ? [...raw]
      : Array.isArray(raw)
        ? raw
        : [raw?.["0"], raw?.["1"], raw?.["2"], raw?.["3"]];
  return `#${bytes
    .slice(0, 3)
    .map((value) => Number(value).toString(16).padStart(2, "0"))
    .join("")}`;
}

function hasBackground(spans: any[], bg: string): boolean {
  return spans.some((span) => colorHex(span, "bg") === bg);
}

function backgrounds(spans: any[]): string[] {
  return [...new Set(spans.map((span) => colorHex(span, "bg")))].sort();
}

function hasAttributes(span: any, mask: number): boolean {
  return (Number(span.attributes ?? 0) & mask) === mask;
}

function expectFrameWithin(frame: string, width: number, height: number): void {
  const lines = frame.replace(/\n$/, "").split("\n");
  expect(lines.length).toBeLessThanOrEqual(height);
  for (const line of lines) expect([...line].length).toBeLessThanOrEqual(width);
}

function lineIndex(frame: string, text: string): number {
  const index = frame.replace(/\n$/, "").split("\n").findIndex((line) => line.includes(text));
  expect(index, `line containing ${text}`).toBeGreaterThanOrEqual(0);
  return index;
}

function lineContaining(frame: string, text: string): string {
  const line = frame.replace(/\n$/, "").split("\n").find((candidate) => candidate.includes(text));
  expect(line, `line containing ${text}`).toBeDefined();
  return line!;
}

function columnOf(frame: string, text: string): number {
  return lineContaining(frame, text).indexOf(text);
}

function textEntry(id: number, content: string, favorite: boolean): Entry {
  return {
    id,
    kind: "text",
    mime: "text/plain",
    content,
    preview: content,
    hash: `text-${id}`.padEnd(64, "0"),
    favorite,
    created_at_ms: Date.now() - id * 1000,
    last_used_at_ms: null,
    byte_len: Buffer.byteLength(content),
    source_app: null,
    blob_path: null,
    image_width: null,
    image_height: null,
  };
}

function imageEntry(id: number, blobPath: string | null = null, byteLength = 2048, mime = "image/png"): Entry {
  return {
    id,
    kind: "image",
    mime,
    content: "hash",
    preview: `${mime} 32x24`,
    hash: `image-${id}`.padEnd(64, "0"),
    favorite: false,
    created_at_ms: Date.now() - id * 1000,
    last_used_at_ms: null,
    byte_len: byteLength,
    source_app: null,
    blob_path: blobPath,
    image_width: 32,
    image_height: 24,
  };
}

function webp2x2Red(): Uint8Array {
  return Uint8Array.from(Buffer.from("UklGRjwAAABXRUJQVlA4IDAAAADQAQCdASoCAAIAAgA0JaACdLoB+AADsAD+8MQL/yC5YXXI1/8gP+QH/ID/+PIAAAA=", "base64"));
}

function rgbaPng(width: number, height: number, pixels: Array<[number, number, number, number]>): Uint8Array {
  const raw = new Uint8Array(height * (1 + width * 4));
  let offset = 0;
  let pixel = 0;
  for (let y = 0; y < height; y += 1) {
    raw[offset++] = 0;
    for (let x = 0; x < width; x += 1) {
      raw.set(pixels[pixel++]!, offset);
      offset += 4;
    }
  }

  return concatBytes(
    Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10]),
    pngChunk("IHDR", concatBytes(u32(width), u32(height), Uint8Array.from([8, 6, 0, 0, 0]))),
    pngChunk("IDAT", deflateSync(raw)),
    pngChunk("IEND", new Uint8Array()),
  );
}

function pngChunk(type: string, data: Uint8Array): Uint8Array {
  const typeBytes = Uint8Array.from([...type].map((char) => char.charCodeAt(0)));
  return concatBytes(u32(data.length), typeBytes, data, Uint8Array.from([0, 0, 0, 0]));
}

function u32(value: number): Uint8Array {
  return Uint8Array.from([(value >>> 24) & 0xff, (value >>> 16) & 0xff, (value >>> 8) & 0xff, value & 0xff]);
}

function concatBytes(...parts: Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((total, part) => total + part.length, 0));
  let offset = 0;
  for (const part of parts) {
    out.set(part, offset);
    offset += part.length;
  }
  return out;
}
