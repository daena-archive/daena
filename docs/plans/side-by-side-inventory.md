# Side-by-side inventory

Factual inventory of the one live location in `src/routes/+page.svelte`, for the side-by-side work described in `docs/plans/tabs.md` ("Later"). Two groups visible at once means two live locations. Today `ShellTabsState.groups` exists (`src/lib/navigation/tabs.ts:23-33`) and the page renders only `activeGroup` (`+page.svelte` tab strip around 8324).

Line numbers are from the working tree while this file was written. Script anchors (`section` at 471, `currentShellLocation` at 2389, `restoreShellLocation` at 3742, `flushAutoSave` at 5468, `selectEntity` at 5768, `saveDocument` at 6500, `onMount` at 7943) stayed put across re-reads. Markup under the toolbar was moving; template ranges below are from the last read of that region. `docs/plans/tabs.md` still says `#shell-tabpanel` is `.app-main` and includes the topbar. The file read here has `GlobalToolbar` (8354-8364) outside a `div#shell-tabpanel.shell-tabpanel` (8365-10680).

The page is about 13,900 lines. The script ends just before `<main>` at 8240. `<style>` starts at 10781.

`ShellLocation` (`src/lib/navigation/history.ts:25-48`) already snapshots section, view, entity id, writing and timeline views, collection query and scroll, pane widths, surface scroll, and module state (language pane, house-tree session). It does not snapshot the document body, field drafts, revisions, dirty flags, autosave timers, conflicts, webview ready flags, or AI dialog state. Inactive tabs are that snapshot. Switching calls `flushAutoSave`, the leave guards, then `restoreShellLocation` (`switchToTab`, 3867-3891).

Categories: **(a)** per-location, **(b)** shell-global, **(c)** unclear.

## 1. Per-location state

The template reads almost every (a) binding. Writers below are the functions that assign them. `$effect` blocks that also write are named.

### Location identity and which surface is open (a)

| Name | Line | Writes | Notes |
| --- | --- | --- | --- |
| `section` | 471 | `switchSection` (3306), `selectSearchResult` (3286), `switchHousesView` (3375), `openTreePerson` (2436), `applyMapPick` (5631), `openMapEntityFromLink` (5686), `reconcileWorkspaceSection` (3337), tree `onOpenHouseEntry` | `currentShellLocation` reads it. |
| `writingView` | 472 | `switchWritingView` (3347); effect 2795 clamps to a real tab id | |
| `timelineView` | 473 | `switchTimelineView` (3361); effect 2795 | |
| `housesView` | 510 | `switchHousesView` (3375) | |
| `languagePane` | 476 | `requestLanguagePane` (1750), `handleLanguageStateEvent` pane case (1785) | Module state in `ShellLocation` is `{ pane }` (`currentModuleState`, 2176). |
| `languagePaneRequest` | 482 | `requestLanguagePane`, `handleLanguageStateEvent` | Plain `let`. In-flight pane the webview has not accepted. |
| `languageWebview` | 478 | `trackLanguageMount` (1726), `handleLanguageStateEvent` ready (1773) | `{ mounted, ready }`. |
| `languageLeave` | 480 | `canLeaveLanguageSection` (1764), `handleLanguageStateEvent`, `trackLanguageMount` | `LanguageLeaveTracker`. One in-flight can-leave. |
| `writingWebview` | 484 | `trackWritingMount` (1831), `handleWritingStateEvent` ready (2086) | |
| `writingReadyCount` | 486 | `handleWritingStateEvent` ready | Bumps the AI-state effect (1945). |
| `writingFullscreen` | 491 | `handleWritingStateEvent` fullscreen (2120), `exitWritingFullscreen` (2007), `trackWritingMount` | Effect 2014 clears it when `selected.id` or `writingWebviewShown` changes. |
| `writingLoadKey`, `writingLoad` | 496, 498 | `postWritingLoad` (1908), `trackWritingMount`, ready handler | Plain `let`. Load token checked by `writingMessageCurrent`. |
| `writingFlush` | 500 | `flushWritingWebview` (1900), ready and flush-reply handlers | `createWritingFlushBroker`. |
| `writingDraftQueue` | 507 | `enqueueWritingDraft` (1884) | Plain `let`. Serializes draft apply. |
| `writingEditablePosted` | 509 | effect 1959, `postWritingLoad`, ready handler | Plain `let`. |
| `loreWikiOpen`, `loreWikiEntityId` | 858, 859 | `openLoreWiki` (3640), `closeLoreWiki` (3667), `openLoreLibrary` (3628), `openProjection` (3602) | |
| `loreGuideHint`, `timelineGuideHint` | 474, 475 | header guide button (9469), `HostGuide` `hintNonce` | Per the open lore or timeline surface. Two lore panes would share one guide id (`LORE_GUIDE_ID`, 10754). |
| `hostView` | 849 | `openHostView` (2456), `leavePluginView` (1513) | |
| `sandboxView` | 850 | `openPluginView` (2575), `openProjection` (3602), `createMap` (2621), `leavePluginView`, `closeProjectionView` (3675) | One slot. Language and Writing are separate mounts, not this slot. |
| `projectHomeOpen` | 450 | `openProjectHome` (3325), every navigation that leaves home | (a) for the home location. Also the gate that hides the workspace. |
| `showSettings` | 670 | `openSettings` (7205), `openProjectCenter` (7220), `closeSettings` (7256), `dismissSettings` (7300), `resetProjectSessionState` | (a) as the settings or project location (`currentShellLocation` 2390-2391). Singleton tab today. |
| `settingsSurface` | 671 | `openSettings`, `openProjectCenter`, `resetProjectSessionState` | `"application"` or `"project"`. |
| `settingsSection` | 672 | `SettingsView` bind, `openSettings` | |
| `projectSection` | 673 | `ProjectCenter` bind, `openProjectCenter` | |

Derived gates, all (a), all read the bindings above: `languagePaneShown` (1583), `writingEntrySelected` (1600), `writingWebviewShown` (1605), `writingFullscreenShown` (1617), `workspaceViewNavShown` (1619), `mapSurfaceOpen` (2620), `embeddedPluginWebviewIds` (1669). `writingEntrySelected` exists so the shell editor never mounts beside the Writing webview (comment 1595-1598).

### Selection, document, fields, autosave (a)

| Name | Line | Writes | Notes |
| --- | --- | --- | --- |
| `selected` | 515 | `selectEntity` (5768), `clearSelection` (7751), `ensureMapEditorOpen` (5609), maps-navigation listener (8092), `createMap` `oncreated` | |
| `selectedLoading`, `selectedLoadError` | 516, 517 | `loadSelectedState` (5478), `clearSelection` | |
| `selectedLoadToken` | 652 | `loadSelectedState`, `resetProjectSessionState` | Plain `let`. Stale loads no-op via `isCurrent`. One counter for the page. |
| `documentBody` | 518 | `loadSelectedState`, `updateDocumentBody` (4177), `clearSelection` | |
| `documentMode` | 519 | `setDocumentMode` (4185) | `"read"` or `"edit"`. |
| `fields` | 520 | `loadSelectedState`, `updateField` (6361), `clearDateField` (4107), `updateInspectorDateField`, `clearSelection` | |
| `dateEditorOpen`, `dateCalendarByField` | 902, 903 | `loadSelectedState`, `openDateEditor` (4104), `hintDateCalendarsFromEras` (6807) | Open date popovers for the selected entry. |
| `relationships` | 521 | `loadSelectedState`, `updateRelationshipField` (6962), `clearSelection` | |
| `eraContexts` | 522 | `loadSelectedState`, `persistDocumentSnapshot` (6475), `refreshAfterEntityMutation` (5079) | |
| `assets` | 525 | `loadSelectedState`, `refreshSelectedAssets` (7031), asset handlers | |
| `assetBusyId` | 526 | `attachAsset`, `handleAssetSave`, `handleAssetDelete`, `handleAssetReplace` | |
| `mapLocations` | 542 | `loadSelectedState`, `refreshSelectedMapLocations` (5590), unlink and edit, maps-state listener | Links for the selected entity. |
| `metadataDialog` | 524 | `openRelationshipMetadata` (6895), Escape handler (984) | (a) tied to `selected`. One dialog. |
| `assetDialog` | 527 | `openAssetDialog` (7062) | Same. |
| `profileEditorOpen` | 529 | profile open and close in the inspector | (a). `modalOverlayOpen` includes it (1653). |
| `fieldsDialogOpen`, `fieldsDialogTab` | 530, 531 | `openFieldsDialog` (6778), `EntityFieldsDialog` bind | (a). |
| `editorFullscreen` | 638 | `setEditorFullscreen` (4182), maps-state `"fullscreen"` (8114), `leavePluginView`, `toggleWorkbenchPane` | Shell editor and map editor share this flag. |
| `editorRef` | 832 | `RichTextEditor` `bind:this` (10169) | One editor. `flushAutoSave` calls `editorRef?.flushPendingChanges()` (5471). |
| `hasUnsavedChanges` | 639 | `markEntryDirty` (4169), `persistDocumentSnapshot`, `selectEntity`, `clearSelection`, `reloadSelectedFromDisk` | |
| `isSaving`, `savedAt`, `saveError` | 635-637 | `saveDocument` (6500), `persistDocumentSnapshot`, `clearSelection` | |
| `autoSaveTimer` | 640 | `scheduleAutoSave` (4158), `cancelAutoSave` (4152) | Plain `let`. One timer. |
| `dateFieldDirtyTimers` | 641 | `scheduleDateFieldDirty` (4135) | Plain record. |
| `saveInFlight`, `saveQueued` | 642, 643 | `saveDocument` | Plain `let`. One in-flight save for the page. |
| `autoSaveFailureCount` | 644 | `markEntryDirty`, `persistDocumentSnapshot` | Plain `let`. |
| `documentRevision` | 645 | `markEntryDirty`, `selectEntity`, `reloadSelectedFromDisk` | Plain `let`. Local edit generation, not the stored revision. |
| `dirtyFieldKeys` | 648 | `markEntryDirty`, `clearDirtyTracking` (4166) | Plain `Set`. |
| `loadedDocumentRevision` | 649 | `loadSelectedState`, `persistDocumentSnapshot`, `overwriteConflict` (5862) | Plain `let`. Sent as `expectedRevision` to `project.saveEntry` (6432). |
| `loadedFieldRevisions`, `loadedStructuredFieldKeys` | 650, 651 | `loadSelectedState`, `persistDocumentSnapshot`, `overwriteConflict` | Plain. |
| `documentConflict`, `conflictDiskBody` | 653, 654 | `persistDocumentSnapshot` conflict path (6480), `reloadConflict` (5854), `overwriteConflict`, `clearSelection` | `writingEditable` (1854) is false while a conflict or `aiRewriteOpen` is set. |
| `entityMutation` / `entityMutationSnapshot` | 533, 532 | `createMutationController`; archive and identity save | One controller. Snapshot is `$state` so the template updates. |
| `savedMutationTimer` | 6544 | `scheduleClearSavedMutation` (6551) | Plain `let`. |

`inspectorChronologyWarnings` (6793) is `$derived` from the selected entry's chronology fields (a).

### Collection, scroll, panes (a), with shared storage (c)

| Name | Line | Writes | Notes |
| --- | --- | --- | --- |
| `collectionQuery` | 551 | effect 610 (reloads `loadCollectionQuery(section)` and clears `expandedGroups`), `applyWorkspaceCollectionLocation` (2321), filter and paging UI, `switchSection` clears `textSearch` | Effect 602 writes `localStorage` key `daena:collection-query:${section}`. Two locations in one section share that key. |
| `collectionQueryRestoring` | 583 | effect 610, `applyWorkspaceCollectionLocation` | Plain `let`. |
| `collectionScopeKey` | 582 | effect 2807 | Plain `let`. Resets `collectionPage` and page index when the scope changes. |
| `expandedGroups` | 548 | `toggleGroup` (3211), effect 610, `applyWorkspaceCollectionLocation` | |
| `filterOpen` | 547 | filter button, `dismissTransientMenus` (3408) | One popover. |
| `collectionPage` | 560 | collection load effect (2828), effect 2807 | |
| `collectionLoading`, `collectionError` | 561, 562 | collection load effect | |
| `collectionRequest` | 563 | collection load effect | Plain `let`. Stale-response token. |
| `collectionRefreshEpoch` | 565 | `bumpCollectionRefresh` | |
| `collectionListElement`, `collectionPaneElement` | 584, 585 | `CollectionPane` binds (9828) | One DOM node each. |
| `contentPaneElement`, `inspectorPaneElement` | 586, 587 | `ContentPane` / `InspectorPane` binds | |
| `collectionScrollBySection` | 588 | `rememberCollectionScroll` (2311), `queueCollectionScroll` (2316) | Keyed by `WorkspaceSection`, not by tab. `ShellLocation` copies one scroll number at capture time. Two live lore locations would share the `"lore"` slot. |
| `pendingCollectionScroll` | 589 | `queueCollectionScroll` | One pending restore. |
| `workbenchPaneVisibility`, `workbenchPaneWidths` | 593, 594 | effect 619 (`loadWorkbenchLayout(section)`), `resizeWorkbenchPane` (2252), `toggleWorkbenchPane` (2277) | Effect 619 also clears `restoredWorkspacePaneDimensions`. Storage key is per section (`src/lib/shell/workbench-persistence.ts:21-23`), one live record. `ShellLocation.panes` stores widths, not visibility. |
| `restoredWorkspacePaneDimensions` | 590 | `restoreWorkspacePaneDimensions` (2224), resize and reset | |
| `workbenchViewportWidth` | 591 | `handleWorkbenchResize` (7993) | (b) the window width. Both panes would share it. |
| `specializedSurfaceElement` | 598 | `SpecializedSurface` `bind:element` | One node. Every full-surface branch binds it. |
| `specializedSurfaceScrollByKey` | 599 | `rememberSpecializedSurfaceScroll` (2368) | Key is `plugin:{id}:{view}` or `workspace:{section}:{view}` (`specializedSurfaceKey`, 2334). Not per tab. |
| `pendingSpecializedSurfaceScroll` | 600 | `restoreSpecializedSurfaceScroll` (2375) | One pending restore. |

Manuscript outline, all (a), all for the writing collection: `manuscriptOutline` (571), `manuscriptOutlineEntities` (572), `manuscriptOutlineRequest` (573, plain), `manuscriptOutlineLoading` (574), `manuscriptOutlineTruncated` (575), `manuscriptOutlineLibraryKey` (576, plain), `manuscriptExpanded` (577), `manuscriptPathLabels` (578), `pendingManuscriptParentId` (579), `writingAppearanceLabels` (580). Writers: `listManuscriptOutlineSummaries` (2960), `refreshWritingAppearanceLabels` (3007), `toggleManuscriptOutlineNode` (3279).

Houses collection, (a) but refreshed from project data: `kinshipRefreshEpoch` (567), `houseCollectionSummaries` (568), `houseSummaryRequest` (569, plain), `houseSummariesPending` (570). `treeRootId` (855), `treeSession` (856), `treeRestoreNonce` (857) are the tree location (`restoreShellLocation` 3767-3772).

### Map editor session (a)

| Name | Line | Writes |
| --- | --- | --- |
| `mapsEditorMode` | 2617 | `createMap`, map `oncreated` |
| `mapsVectorStart` | 2618 | `createMap` |
| `mapProviderMenuOpen` | 2619 | header and empty-state menus |
| `mapsEditorKey` | 657 | `ensureMapEditorOpen`, maps-navigation listener, `oncreated`, `resetProjectSessionState` |
| `mapReloadCounter` | 656 | `reloadMapOriginal` (2754) |
| `mapSaveStates` | 655 | maps-state listener (8112), `PhysicalMapEditor` / `NativeVectorMapEditor` `onstate` | Record keyed by map id. Can hold many maps. The open editor is still one (`mapsEditorKey`, one `sandboxView`). |
| `mapRecoveryBusy` | 658 | `restoreMapDraft` (2759) |
| `mapFocusLinkId`, `mapFocusFeatureId` | 659, 660 | maps-navigation listener (8099) |
| `mapSelection` | 661 | maps-selection listener (8177), only if `mapEntityId === currentMapId()` |
| `mapPickPending`, `mapPickNotice` | 662, 668 | `beginMapPick` (5621), `applyMapPick` (5631), maps-state pick events |
| `mapReconcileNotice` | 667 | `openMapLocation` (5594), maps-state `"reconcile"` |
| `savedMapsCache`, `savedMapsRequest` | 2668, 2669 | `savedMaps` (2741), `oncreated` | (c) project map list cache used by the maps location. One cache. |

`nativeVectorSession()` is a module singleton (`src/lib/maps/native-vector/session.ts:7-14`). `resolveDirtyMapSession` (1480) calls it for the one open map.

### AI dialog bound to the selection (a)

Settings AI (`aiSettings` 790, `aiPromptTemplates` 802, `aiStatus` 803, `aiModels` 804, `aiIndexStatus` 809, `remoteCredential` 814, `aiUsage` 815, `aiProvider` derived 885) is (b).

Selection session (a): `aiRewriteOpen` (816), `aiBusy` (817), `aiCancelPending` (818), `aiMode` (819), `aiRequestId` (820), `aiInstruction` (821), `aiStreamText` (822), `aiPreviewOutput` (823), `aiProgressMessage` (824), `aiSourceSelection` (825), `aiSourceSelectionPlain` (826), `aiGenerationContext` (827), `aiSourceBody` (828), `aiSourceRevision` (829), `aiLastSequence` (830), `aiUnlisten` (831), `aiStartCancelled` (843), `aiSourceEntityId` (845). Writers: `setAiSelection` (4425), `openAiAction` (4430), `startAiRewrite` (4744), `closeAiRewrite` (4666). `loadSelectedState` closes both AI dialogs (5485-5486).

Field fill (a): `aiFieldFillBusy` (837), `aiFieldFillOpen` (838), `aiFieldFillRequestId` (839), `aiFieldFillStream` (840), `aiFieldSuggestions` (841), `aiFieldUnlisten` (842), `aiFieldFillStartCancelled` (844), `aiFieldFillEntityId` (846), `aiFieldFillLastSequence` (847). Writers: `fillAiFields` (4528), `closeAiFieldFill` (4453). `handleAiFieldFillEvent` (4465) drops events whose entity is not `selected`.

`languageListRefreshed` (2729, plain) is a one-shot for the language collection (a).

### Create dialog (b), reads the live section

`name` (628, create title, `bind:value` in the create dialog), `selectedCreateKey` (629), `createFieldValues` (630), `createDateCalendarByField` (631), `createDocumentBody` (632), `showDiscardPrompt` (633), `pendingCreateDiscard` (634), `createEraContexts` (523), `showCreateForm` (896), `createDialogView` (897), `createMoreDetailsOpen` (898), `createBusy` (899), `createDialogElement` (900), `createDialogReturnFocus` (901, plain). Writers: `activateCreateOption` (6046), `createWithOption` (5897), `createEntity` (6021), `closeCreateForm` (6028), `hasCreateValues` (1382). `createOptions` follows `section` via `activeModuleId` (1002). One dialog. Opening it does not belong to a second pane unless the dialog is told which location it creates into.

### Shell-global (b)

Project and session: `ready` (446), `error` (447, also drawn inside the writing column), `projectTransitionBusy` (448), `projectTransitionMessage` (449), `projectInfo` (884), `projectStatusEpoch` (888, plain), `gitStatus` (889), `projectDiagnostics` (669, blocks save), `modules` (543), `entities` (514, the entity cache both panes must share), `calendarDefinitions` (511), `calendarEntities` (513), `archivedCount` (674), `recentProjects` (892), `globalQuery` (544), `searchMatches` (893), `mapFeatureMatches` (894), `searchRequest` (895, plain), `quickOpenOpen` (545), `quickOpenSearchLoading` (546).

Chrome: `themePreference` (433), `themePack` (434), `projectThemePack` (435), `themePacks` (436), `updateChannelPreference` (438), `railCollapsed` (549), `showProjectMenu` (890), `statusCenterOpen` (675), `tabStripPopoverOpen` (676), `railMenuOpen` (677), `popoverOverlapsWebview` (1666), `displayVersion` (696), `shellTabs` (457), `shellNavigationBusy` (469), `shellNavigationRestoring` (470, plain). `shellNavigationHistory` (458) is `$derived` from `activeTab(shellTabs).history`. It is already per active tab, and it is the only history the toolbar and `navigateShellHistory` (3803) use. Two visible groups need two of these. Category (c) until a focused group exists.

Tabs and menus that are not a location: `lifecycleToast` (540), `lifecycleToastTimer` (541, plain). `showExternalImport` (891).

Plugin admin and schema editor, shown inside the project location but they mutate the project: `adminPlugins` (848), `adminBusy` (860), `pluginActionId` (861), `installing` (862), `installConsent` (863), `installReview` (864), `installSummary` (865), `registryCatalog` (866), `registryBusy` (867), `registryError` (868), `upgradePreview` (869), `upgradeBusy` (870), `confirmAction` (871), `confirmBusy` (879), `deleteTarget` (880), `deleteInput` (881), `deleteBusy` (882), `deleteBackupPath` (883), `moduleSchemaOverlay` (678), `moduleSchemaPackage` (679), `moduleSchemaBusy` (683), `moduleSchemaMessage` (684), `moduleSchemaRevision` (685), `moduleSchemaContentRevision` (686), `moduleSchemaConflict` (687), `moduleSchemaSaveRequestId` (688), `schemaPluginId` (689), `schemaPluginName` (690), `schemaEditorDirty` (691), `schemaOverlayLoadToken` (692, plain), `schemaOverlayCache` (693), `schemaEntityCountsByType` (694), `schemaEntityCountsLoaded` (695).

`modalOverlayOpen` (1636) and `anchoredPopoverOpen` (1664) are `$derived` over both shell dialogs and location dialogs (AI, fields, metadata, asset, editor fullscreen). One flag hides every embedded webview (`setPluginWebviewVisibleQuietly`, 1678-1722).

`entityEditDialog` (528) is (c). One modal. `openEntityEditDialog` (6520) flushes and may `selectEntity` the target first. It is not a second location, and it assumes the one `selected`.

`error` is (c) in the same way: one string, toast at 10659, and the writing column at 9861.

## 2. Per-location functions

Each row is the function and the (a) state it touches. Shell flags it also sets (`projectHomeOpen`, `error`) are omitted unless they are the point.

| Function | Line | State |
| --- | --- | --- |
| `currentShellLocation` | 2389 | Reads `showSettings`, `settingsSurface`, `projectSection`, `settingsSection`, `projectHomeOpen`, `hostView`, `sandboxView`, `section`, `selected`, `writingView`, `timelineView`, `housesView`, `treeRootId`, collection, panes, surface scroll, `languagePane`, `treeSession`. |
| `currentWorkspaceLocationView` | 2161 | Reads `loreWikiOpen`, projection sandbox, `timelineView`, `writingView`, `housesView`. |
| `currentModuleState` / `restoreModuleState` | 2176, 2182 | `languagePane`, `treeSession`. |
| `currentWorkspaceCollectionLocation` / `applyWorkspaceCollectionLocation` | 2192, 2321 | `collectionQuery`, `expandedGroups`, scroll. |
| `currentWorkspacePaneDimensions` / `restoreWorkspacePaneDimensions` | 2214, 2224 | Pane elements, `restoredWorkspacePaneDimensions`. |
| `rememberCollectionScroll` / `queueCollectionScroll` | 2311, 2316 | `collectionScrollBySection`, `pendingCollectionScroll`. |
| `rememberSpecializedSurfaceScroll` / `restoreSpecializedSurfaceScroll` | 2368, 2375 | Surface scroll map and element. |
| `recordCurrentShellLocation` / `recordShellDeparture` | 2426, 2431 | Active tab history, via `shellNavigationRestoring`. |
| `restoreShellEntity` | 3719 | `entities` cache, `selectEntity` or `clearSelection`. |
| `restoreShellLocation` | 3742 | Sets `shellNavigationRestoring`. Dispatches to `openProjectHome`, `openSettings`, `openProjectCenter`, `switchSection`, `openLoreLibrary`, `switchTimelineView`, `switchWritingView`, `switchHousesView`, `restoreModuleState`, `applyWorkspaceCollectionLocation`, `restoreShellEntity`, `openWorkspaceView`, pane and scroll restore. Sets `treeRootId`, `treeRestoreNonce`. |
| `navigateShellHistory` | 3803 | `flushAutoSave`, `canLeaveLanguageSection`, then `restoreShellLocation`. One history. |
| `switchToTab` / `openInNewTab` / `applyTabsChange` | 3867, 3901, 3956 | Same leave sequence, then restore. `openInNewTab` dedupes with `tabIdentity` across every group (`otherTabByIdentity`, 3856). |
| `snapshotActiveTab` | 3828 | Writes the active tab's `location` from `currentShellLocation`. |
| `selectEntity` | 5768 | `flushAutoSave`, language leave, map `leavePluginView`, then `selected`, `editorFullscreen`, `hasUnsavedChanges`, `documentConflict`, `documentRevision`, dirty set, `saveError`, `loadSelectedState`. Same id returns immediately (5769), so a second pane cannot select the entity the first pane holds. |
| `openEntityById` | 5796 | Cache, then `selectSearchResult` from Language, else `selectEntity`. |
| `selectSearchResult` | 3286 | Section change, `leavePluginView`, `selectEntity`. |
| `openEntityWithIntent` | 5763 | `openInNewTab` or `selectEntity`. Wired to `setShellEntityOpener` (396) and Quick Open. |
| `switchSection` | 3306 | Flush, language leave, `leavePluginView`, `section`, collection scroll, `clearSelection` when the section changes. |
| `switchWritingView` / `switchTimelineView` / `switchHousesView` / `switchLanguagePane` | 3347, 3361, 3375, 3395 | Flush, leave, the view flag, usually `clearSelection`. |
| `openProjectHome` | 3325 | Flush, language leave, `leavePluginView`, `projectHomeOpen`. |
| `openLoreLibrary` / `openLoreWiki` / `closeLoreWiki` / `openProjection` / `closeProjectionView` / `openWorkspaceView` | 3628, 3640, 3667, 3602, 3675, 3689 | Wiki and sandbox flags. |
| `openHostView` / `openPluginView` / `closePluginView` / `leavePluginView` | 2456, 2575, 3682, 1513 | `hostView`, `sandboxView`, `editorFullscreen`. `leavePluginView` calls `resolveDirtyMapSession` then `closeNativePluginWebviews` when a full surface was open. |
| `closeNativePluginWebviews` | 1472 | `project.closeAllPluginWebviews()`. Every plugin webview, not one instance. |
| `resolveDirtyMapSession` | 1480 | `mapSaveStates`, `mapsEditorMode`, `nativeVectorSession`. |
| `canLeaveLanguageSection` | 1764 | `languageWebview`, `languageLeave`. Posts `language.can-leave` to `LANGUAGE_PLUGIN_ID`. |
| `requestLanguagePane` / `postLanguageMessage` / `handleLanguageStateEvent` / `trackLanguageMount` | 1750, 1736, 1773, 1726 | Language pane and webview. `open-entity` calls `openEntityById` or `openInNewTab`. |
| `postWritingMessage` / `applyWritingDraft` / `enqueueWritingDraft` / `flushWritingWebview` / `postWritingLoad` / `notifyWritingSaved` / `handleWritingConflictAction` / `handleWritingStateEvent` / `trackWritingMount` / `exitWritingFullscreen` | 1844, 1874, 1884, 1900, 1908, 1984, 1990, 2086, 1831, 2007 | Writing webview, `documentBody` via `updateDocumentBody`, conflict, fullscreen. Drafts are checked against `selected` and `writingLoad`. |
| `flushAutoSave` | 5468 | `flushWritingWebview`, `editorRef`, `flushDateFieldDirty`, `cancelAutoSave`, then `saveDocument` if `hasUnsavedChanges`. |
| `loadSelectedState` | 5478 | Clears and refills document, fields, relationships, eras, assets, map locations, revisions. Closes both AI dialogs. |
| `updateDocumentBody` / `updateField` / `markEntryDirty` / `scheduleAutoSave` / `setDocumentMode` | 4177, 6361, 4169, 4158, 4185 | Document and dirty state. |
| `persistDocumentSnapshot` | 6382 | Calls `project.saveEntry` with `expectedRevision: loadedDocumentRevision` (6407-6432). On success updates revisions and `hasUnsavedChanges` only if `documentRevision` and `selected.id` still match. Conflict sets `documentConflict`. |
| `saveDocument` | 6500 | One `saveInFlight`. There is no function named `saveEntry` on the page. |
| `reloadSelectedFromDisk` / `reloadConflict` / `overwriteConflict` / `saveConflictRecoveryCopy` | 5833, 5854, 5862, 5887 | Selection load, conflict, recovery copy of `documentBody`. |
| `clearSelection` | 7751 | Cancels autosave and date timers. Clears selection, document, fields, relationships, assets, conflict, create form. Does not clear `sandboxView` or `section`. |
| `resetProjectSessionState` | 7776 | `clearSelection`, new `shellTabs`, scroll, home, map session, settings flags, `ready = false`. Used by `closeProject` (5455) after one `flushAutoSave` and `leavePluginView({ closeWebviews: true })`. |
| `refreshAfterEntityMutation` | 5054 | Entity cache, `bumpCollectionRefresh`, kinship epoch, and `eraContexts` if the open entry links that era. One `selected`. |
| `ensureMapEditorOpen` / `beginMapPick` / `applyMapPick` / `openMapEntityFromLink` | 5609, 5621, 5631, 5686 | Selection, section, map pick, `openPluginView`. |
| `runShellShortcut` / `runTabShortcut` | 2053, 2021 | History, Quick Open, create, and tabs of `activeGroup` only (2034). |
| `handleHistoryKey` / `handleTabKey` / `handleQuickOpenKey` / `handleWritingFullscreenKey` | 7959, 7983, 7971, 7989 | Window listeners. No pane target. |
| `onKey` (capture, 995) | ~900-995 | Escape closes the one stack of dialogs, including `aiRewriteOpen`, `metadataDialog`, `assetDialog`, `fieldsDialogOpen`. |

Effects that are part of the location, not just functions: collection query persist and reload (602, 610), workbench layout reload on `section` (619), writing load (1932), writing AI state (1945), writing editable (1959), writing conflict (1969), writing save error (1976), writing fullscreen reset (2014), collection fetch (2828).

Window close (`onCloseRequested`, 8022) runs one `flushAutoSave`, one `canLeaveLanguageSection`, one `leavePluginView({ closeWebviews: true })`, then `persistShellTabs`. `beforeunload` (7952) persists tabs and calls `editorRef?.flushPendingChanges()`, and blocks unload only when the one `hasUnsavedChanges` is set. It does not flush the Writing webview.

## 3. Template

Chrome that stays one shell, above the live location:

- `<main>` 8240. Project-transition backdrop 8241. `AppSidebar` 8253-8291 (`railCollapsed`, navigation active from the one `section`).
- Tab strip snippet and `GlobalToolbar` 8324-8364. One strip: `activeGroup(shellTabs)`. Back and forward call `navigateShellHistory`. `panelId="shell-tabpanel"` (8330).

Live location, inside `#shell-tabpanel` (8365-10680). One if/else chain:

| Range | What | Components and the per-location props |
| --- | --- | --- |
| 8370-8377 | Quick Open overlay | `QuickOpen`. Shell (b), but `selectQuickOpenItem` navigates the one location. |
| 8378+ | Create dialog | Create state (b). `id="create-dialog-title"`, `id="new-entity"`, `id="create-document"`. |
| 8727-8733 | View switcher snippet | `WorkspaceViewNav`: `section`, `currentWorkspaceLocationView()`, `openWorkspaceView`. |
| 8734-8753 | Settings location | `SettingsView` `bind:section={settingsSection}`. |
| 8754-9126 | Project location | `ProjectCenter` `bind:section={projectSection}`. Snippets: `SchemaSettingsPanel`, `ProjectAiSettingsPanel`, `GitSettingsPanel` (`beforeWrite={flushAutoSave}`). |
| 9127-9144 | No project | Welcome. Not a location. |
| 9145-9172 | Home | `ProjectHome`. Opens workspaces through `switchSection` and entities through `selectSearchResult`. |
| 9173-9183 | Host plugin | `SpecializedSurface` (scroll bind) + `HostView` `plugin` / `view`. |
| 9184-9207 | Lore wiki | `SpecializedSurface` + `WikiView`: `loreWikiEntityId`, `onSelectEntity` → `selectEntity`, `onOpenEntityInNewTab`. |
| 9208-9303 | House tree | `SpecializedSurface` + `TreeSurface`: `treeRootId`, `treeSession`, `treeRestoreNonce`, `kinshipRefreshEpoch`. |
| 9304-9322 | Lore graph or chronology | `SpecializedSurface` + `SandboxView` `pluginId={projection.pluginId}` and no `viewId`. |
| 9323-9335 | Other plugin sandbox | `SandboxView` `pluginId` and `viewId` from `sandboxView`. |
| 9336-9349 | No workspace modules | Empty state. |
| 9350-10658 | Workbench | The default location. |

Workbench (9480-10657, `class="workspace-grid"`, `style={workspaceGridStyle()}`):

- `WorkspaceHeader` 9462: kicker, title, description from `section`; guide bumps `loreGuideHint` or `timelineGuideHint`; pane toggles.
- `CollectionPane` 9815: query, outline, scroll binds, `mapSurfaceOpen`. Collection controls (9488) bind `collectionQuery` and use radio `name="sortField"` and `name="pageSize"`.
- `PaneResizeHandle` 9832 and 10244.
- Language column 9840: `SandboxView` `pluginId="daena.language"`, `viewId="language-workspace"`, `entityId={selected?.id}`, `moduleState={{ pane: languagePane }}`. `{#key selected?.id}` remounts on selection change. `{@attach trackLanguageMount}`.
- Writing column 9851: `SandboxView` `pluginId="daena.writing"`, `viewId="writing-workspace"`. `{@attach trackWritingMount}`.
- Else content 9871, only when `!writingEntrySelected`: `ContentPane` 10237 (`editorFullscreen`, map-editor flag, `contentPaneElement`). Body: map editors (`PhysicalMapEditor`, `NativeVectorMapEditor`) when `sandboxView.renderer === "maps"`, otherwise the shell editor. `StatusSummary` gets `selectedLoading`, `saveError`, `hasUnsavedChanges`, `savedAt`, `isSaving`. `MarkdownArticle` (10159) or `RichTextEditor` (10168) gets `documentBody`, `editorRef`, `documentMode`, AI callbacks. `EntityArchiveAction` gets `selected`.
- Inspector 10253: `InspectorPane` when `selected`, else an empty `InspectorPane` (10654). `InspectorOverview`, `InspectorSection`, `RelationshipPicker`, `InspectorDateField`, `CalendarEditor`, `ProfileEditor`, `ProfileEventChanges`, map-link actions. `EntityFieldsDialog` 10641 when `fieldsDialogOpen`. Props are `selected`, `fields`, `relationships`, `eraContexts`, `assets`, `mapLocations`, AI field-fill state.

Still inside the panel, one toast each: `error` (10659), `lifecycleToast` (10667).

Outside the panel, still one: `EntityHoverCard` (10682, `openEntityWithIntent`), relationship metadata dialog (10693), `AssetDialog` (10703), `ExternalImportDialog` (10712, `clearSelection` on commit), `EntityIdentityDialog` (10723), two `HostGuide`s (10753, 10766) gated on the one `section`, `DialogHost` (10779).

## 4. Cross-location hazards

**Same entity, two editors.** `persistDocumentSnapshot` sends one `loadedDocumentRevision` as `expectedRevision` (6432). Two stores can save different entities. Two stores on the same entity both start from the same revision; the second save gets the conflict path (6480) and sets the one `documentConflict`. `selectEntity` returns immediately when `selected.id` is already that id (5769), so the page cannot hold the entity twice. `saveInFlight` (642) is one promise. `selectedLoadToken` is one counter. Writing's comment (1869-1872) says the webview cannot write: `applyWritingDraft` goes through `updateDocumentBody` and the one autosave. The plan's rule ("single writer per document, not per shell") is not how the code works today. The shell is the only writer, and it has one document.

**`tabIdentity` collapses duplicates.** `src/lib/navigation/tabs.ts:87-99` ignores scroll and query. `findTabByIdentity` (102) walks every group. `openInNewTab` (3932-3946) switches to that tab instead of keeping a second copy. A copy of the current tab is the exception (3936). Side-by-side duplicates need a different identity, or a same-document lock.

**Keyboard and Quick Open assume one target.** `handleHistoryKey` (7959) and `handleTabKey` (7983) call `runShellShortcut`. Tab shortcuts move inside `activeGroup` only (`runTabShortcut`, 2034). Back and forward use `shellNavigationHistory` from the active tab (458, 3803). The toolbar is one pair of buttons (8362-8363). Quick Open (`selectQuickOpenItem`, 6312) and the sidebar (`openSidebarNavigationItem`, 2538) navigate that same location. `setShellEntityOpener` (396, `src/lib/editor/shell-host.ts:118-122`) installs one `openEntity` on `shellEditorHost`. Every `RichTextEditor` that uses the default host (`installShellEditorHost`, 395; `setDefaultEditorHost`, `src/lib/editor/host.ts:58`) opens into that callback. `HostGuide` is active when `section === "lore"` or `"timeline"` (10755, 10768), not when a particular pane is focused.

**Plugin events are routed by plugin id.** `postLanguageMessage` and `postWritingMessage` call `project.postPluginWebviewMessage(LANGUAGE_PLUGIN_ID | WRITING_PLUGIN_ID, …)` (1738, 1846). Listeners are global event names: `language-state` (8185), `writing-state` (8202), `maps-navigation` (8076), `maps-state` (8112), `maps-selection` (8177), `shell-shortcut` (8194). None carry an instance id. `maps-state` does carry `mapEntityId`, and the handler still writes the one `selected`, `editorFullscreen`, and `mapLocations` of `selected`.

**One embedded webview.** `plugin_window_label` is `plugin:{sanitizedId}` with no instance (`crates/daena-plugin-host/src/runtime.rs:64-75`). Opening a webview closes every other embedded `plugin:` label (`src-tauri/src/plugin_webview.rs:300-311`) and recreates its own label (314-326). `closeNativePluginWebviews` closes all of them (1472). `embeddedPluginWebviewIds` (1669) is one set. The comment at 1686-1689 says Language, Writing, and sandbox views are exclusive branches, so one overlap test against `.sandbox-view-container` covers every webview. `measureAnchoredPopovers` (1691) uses `document.querySelectorAll("[data-shell-popover]")` and `document.querySelectorAll(".sandbox-view-container")`. `SandboxView` (`src/lib/plugins/SandboxView.svelte:58`) resizes by `pluginId` only. Two Language panes, or Lore graph beside Chronology (`daena.lore` and `daena.timeline`), or a graph beside Writing, cannot exist until labels are `plugin:{id}:{instance}` as `tabs.md` describes under keep-alive.

**DOM queries and ids that assume one node.** `document.getElementById("new-entity")` (6064) and `document.getElementById("entity-edit-name")` (6536). `id="shell-tabpanel"` (8366) and `aria-labelledby={`shell-tab-${id}`}` (8368). One `panelId`. Collection radios `name="sortField"` and `name="pageSize"` (collection controls, 9488) are document-scoped; a second filter joins the same radio group. Create-dialog ids (`create-dialog-title`, `new-entity`, `create-document`, `discard-create-title`) stay unique only while there is one create dialog. The content-column AI panel (above `MarkdownArticle` at 10159) uses `id="ai-rewrite-title"`. `collectionListElement?.querySelector` (6658) and `createDialogElement?.querySelector` (6127) are scoped to one element; a second unscoped `querySelector` is the popover measurement above. `specializedSurfaceElement` is one bind; two `SpecializedSurface`s would fight over it.

**One flush on close.** `closeProject` (5455): one `flushAutoSave`, one `leavePluginView({ closeWebviews: true })`, `persistShellTabs`, `resetProjectSessionState`, `project.close`. Window close (8022) is the same sequence plus `canLeaveLanguageSection`. `beforeunload` (7952) only sees `hasUnsavedChanges` and `editorRef`. A second dirty document would not block unload and would not be saved.

**Section-keyed effects clobber a second store.** Effect 610 replaces `collectionQuery` and clears `expandedGroups` whenever `section` changes. Effect 619 replaces pane visibility and widths and clears restored dimensions. Effect 602 persists the query under the section key. Those are correct for one live location. They are wrong if two locations exist and either effect stays on the page.

**Leave dialogs are one global.** `resolveDirtyMapSession` and `confirmWritingArchive` call `confirmDialog` (`src/lib/dialogs.svelte`, used at 1484 and 2076). `modalOverlayOpen` includes `dialogState.open` (1655) and hides every webview.

## 5. Existing seams

Already outside the page, and usable as the store's pieces:

- `src/lib/navigation/history.ts`. `ShellLocation`, `recordShellLocation`, `shellHistoryBack`, `shellHistoryForward`. The snapshot the store would save and restore. Pane visibility is not in it.
- `src/lib/navigation/tabs.ts`. Groups, `activeGroup`, `activeTab`, `tabIdentity`, `findTabByIdentity`. Groups are the side-by-side slot. Identity is the duplicate-tab rule. The page only renders `activeGroup`.
- `src/lib/navigation/tab-shortcuts.ts` and `open-intent.ts`. Shortcut names and `OpenIntent` (`"current" | "new-tab"`). No `"other-group"` target.
- `src/lib/shell/workbench-persistence.ts`. `loadWorkbenchLayout` / `saveWorkbenchLayout` / `loadCollectionQuery`, keyed by section, not by tab.
- `src/lib/modules/language-bridge.ts` and `src/lib/modules/writing-bridge.ts`. Pure parsers, `LanguageLeaveTracker`, `createWritingFlushBroker`, message builders. The page holds one tracker and one broker and posts to a fixed plugin id. The brokers are already per call; the page constructs one of each (480, 500).
- `src/lib/modules/plugin-leave.ts`. Leave timers used by the language tracker.
- `src/lib/entity-lifecycle/mutationState.ts`. `createMutationController` is per call. The page creates one (533).
- `src/lib/entity-lifecycle/asyncEntityQuery.ts`. Paging helpers. `searchEntitiesPaged` (5086) closes over the page.
- `src/lib/fields/persistence.ts` and `src/lib/fields/values.ts`. Field dirty and revision helpers. No store.
- `src/lib/editor/RichTextEditor.svelte`. Already one component instance per mount (`bind:this={editorRef}`). The host it calls is not per instance.
- `src/lib/editor/shell-host.ts` and `host.ts`. One `shellEditorHost`, one `defaultHost`.
- `src/lib/editor/reactive-context.svelte.ts`. `insideReactiveUpdate`. Not location state.
- `src/lib/plugins/SandboxView.svelte`. Per mount for bounds observation. IPC is `resizePluginWebview(pluginId, bounds)` and `postPluginWebviewMessage(pluginId)`.
- `src/lib/maps/native-vector/session.ts`. One registered session.
- `src/lib/shell/SpecializedSurface.svelte`, `CollectionPane.svelte`, `ContentPane.svelte`, `InspectorPane.svelte`. Presentational. Safe to mount twice if each gets its own binds and they do not use document-level ids.
- `src/lib/dialogs.svelte` / `DialogHost.svelte`. One dialog stack.

Not a seam: the entity cache `entities`, `modules`, calendars, and `project` IPC. Those stay one per project. `refreshAfterEntityMutation` must fan out to every location whose `selected.id` matches, instead of the one `selected`.

## 6. Size and extraction order

Rough size of what moves:

- Declarations of (a) state, including the plain `let`s in section 1: about 300 lines, scattered from 450-903, 2617-2669, and the save tokens at 640-652.
- Functions and effects that load, save, navigate, or tear down that state: about 2,900 lines. The dense blocks are plugin leave and the language and writing bridges (1472-2136, ~665), location capture (2161-2424, ~264), view switches and restore (3286-3979, ~690), dirty and autosave (4103-4189, ~87), selection AI (4425-4900, ~475), load, select, and conflict (5468-5895, ~428), `persistDocumentSnapshot` and `saveDocument` (6361-6519, ~159), plus the map listeners inside `onMount` (8076-8179).
- The live-location template, 8734-10680: about 1,950 lines. It has to be mounted once per group. The chrome above 8365 stays one.
- CSS from 10781 to the end of the file stays. It is shared classes. Selectors that assume one `.workspace-grid` or one `.sandbox-view-container` need a check when two grids exist, not a move.

Order that leaves the app working after each step:

1. **One `LocationStore`, one caller.** Move the (a) state and the functions in section 2 into a store the page constructs once. `currentShellLocation`, `restoreShellLocation`, `selectEntity`, `flushAutoSave`, and `saveDocument` become methods. The template still reads the store. No second pane. Behaviour stays the same, including the section-keyed effects (602, 610, 619), which move with the store so they still key off that one `section`.
2. **One `LocationView`.** Move the chain from 8734 through 10680 into a component that takes the store. The page still mounts it once, under the one `#shell-tabpanel`. Chrome, Quick Open, create dialog, and `DialogHost` stay on the page.
3. **Stop the store from using section-global slots.** Give the store its own collection query, expanded groups, scroll, and pane visibility, and stop writing `daena:collection-query:${section}` and `daena:workbench-layout:${section}` from a second instance. `ShellLocation` remains the snapshot. Still one instance, so this is a storage-key change only where a second instance would have collided. Do this before mounting two, or the second mount will reset the first through effects 610 and 619.
4. **Mount two `LocationView`s.** One per visible group. Each group keeps the strip and history it already has. Focus chooses which group receives `handleHistoryKey`, `handleTabKey`, Quick Open, and `setShellEntityOpener`. Refuse a second location that `tabIdentity` already has open, until the same-document rule exists. Refuse a second embedded webview: Language, Writing, graph, chronology, and maps stay single until step 5. Two shell-editor locations (two lore entries, lore beside a timeline entry) are the first pair that can work.
5. **Same document, then webviews.** If both sides may show one entity, each store keeps its own `loadedDocumentRevision` and `saveInFlight`, and the second save must take the conflict path on purpose. Writing stays one writer per document. Webviews need `plugin:{id}:{instance}` labels, host messages addressed to an instance, and `listen` payloads that name the instance. That is the keep-alive work in `tabs.md`, and side-by-side of two Writing tabs depends on it. `closeProject` and the window-close handler flush every store, and `beforeunload` blocks when any store is dirty.

Step 1 is the one that matches the plan sentence: the selected entity, fields, autosave, and document state leave `+page.svelte` with no behaviour change. Rendering two is step 4, after the store has been the only live location.
