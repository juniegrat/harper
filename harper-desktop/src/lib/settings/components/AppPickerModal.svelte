<script lang="ts">
import { onMount } from 'svelte';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { type AppSearchResult, Client } from '$lib/client';
import AppIcon from './AppIcon.svelte';

export let bundleId = '';
export let existingBundleIds: string[];
export let isSaving = false;
export let close: () => void;
export let add: (bundleId: string) => void;

let searchResults: AppSearchResult[] = [];
let isSearching = false;
let isDraggingApp = false;
let debounceTimeout: number | null = null;
let searchRequestId = 0;

$: trimmedBundleId = bundleId.trim();
$: isDuplicate = existingBundleIds.includes(trimmedBundleId);
$: canAdd = Boolean(trimmedBundleId) && !isDuplicate && !isSaving;

onMount(() => {
	let mounted = true;
	let unlistenDragDrop: (() => void) | undefined;

	const initialSearch = window.setTimeout(() => {
		void performSearch(bundleId);
	}, 0);

	getCurrentWebviewWindow()
		.onDragDropEvent((event) => {
			if (!mounted) return;

			if (event.payload.type === 'enter' || event.payload.type === 'over') {
				isDraggingApp = event.payload.paths.some((p) => p.endsWith('.app'));
			} else if (event.payload.type === 'leave') {
				isDraggingApp = false;
			} else if (event.payload.type === 'drop') {
				isDraggingApp = false;
				const appPath = event.payload.paths.find((p) => p.endsWith('.app'));
				if (appPath) {
					void addFromPath(appPath);
				}
			}
		})
		.then((unlisten) => {
			if (mounted) {
				unlistenDragDrop = unlisten;
			} else {
				unlisten();
			}
		});

	return () => {
		mounted = false;
		unlistenDragDrop?.();
		window.clearTimeout(initialSearch);
	};
});

async function addFromPath(path: string) {
	try {
		const result = await Client.appFromPath(path);
		bundleId = result.bundle_id;
		searchResults = [result];
	} catch (error) {
		console.error('Could not read dropped app:', error);
	}
}

async function performSearch(query: string) {
	const requestId = ++searchRequestId;

	isSearching = true;
	try {
		const results = await Client.searchApps(query);

		if (requestId === searchRequestId && query === bundleId) {
			searchResults = results;
		}
	} catch (error) {
		console.error('Search failed:', error);

		if (requestId === searchRequestId) {
			searchResults = [];
		}
	} finally {
		if (requestId === searchRequestId) {
			isSearching = false;
		}
	}
}

function handleInput(event: Event) {
	const target = event.target as HTMLInputElement;
	bundleId = target.value;

	if (debounceTimeout) {
		clearTimeout(debounceTimeout);
	}

	debounceTimeout = window.setTimeout(() => {
		performSearch(bundleId);
	}, 300);
}

function selectApp(selectedBundleId: string) {
	bundleId = selectedBundleId;
	void performSearch(bundleId);
}

function submit() {
	if (canAdd) {
		add(trimmedBundleId);
	}
}
</script>

<div
  class="modal-backdrop"
  role="button"
  tabindex="0"
  aria-label="Close application picker"
  on:click={close}
  on:keydown={(event) => {
    if (event.key === "Escape" || event.key === "Enter" || event.key === " ") {
      close();
    }
  }}
>
  <div
    class="modal"
    role="dialog"
    tabindex="-1"
    aria-label="Choose an application"
    on:click|stopPropagation={() => {}}
    on:keydown|stopPropagation={(event) => {
      if (event.key === "Escape") {
        close();
      }
    }}
  >
    <div class="modal-head">
      <strong>Add application</strong>
      <span>Enter the app bundle ID Harper should watch.</span>
    </div>
    <div class="modal-search">
      <span class="settings-icon icon-search" aria-hidden="true"></span>
      <input
        type="text"
        placeholder="Search for an app..."
        value={bundleId}
        disabled={isSaving}
        on:input={handleInput}
        on:keydown={(event) => {
          if (event.key === "Enter") {
            submit();
          }
        }}
      />
    </div>
    <div class="modal-drop-hint" class:dragging={isDraggingApp}>
      {isDraggingApp ? "Drop the .app to add it" : "Tip: drag an .app from Finder onto this window, or paste a bundle ID"}
    </div>
    <div class="modal-list">
      {#if isSearching}
        <div class="empty">Searching...</div>
      {:else if searchResults.length > 0}
        {#each searchResults as result}
          <div
            class="app-result"
            role="button"
            tabindex="0"
            on:click={() => selectApp(result.bundle_id)}
            on:keydown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                selectApp(result.bundle_id);
              }
            }}
          >
            <AppIcon bundleId={result.bundle_id} name={result.name} />
            <div class="app-result-copy">
              <div class="app-result-name">{result.name}</div>
              <div class="app-result-bundle-id">{result.bundle_id}</div>
            </div>
          </div>
        {/each}
      {:else if trimmedBundleId}
        {#if isDuplicate}
          <div class="empty">That application is already configured.</div>
        {:else}
          <div class="empty">No matching apps found. Try typing the bundle ID directly (e.g., com.apple.TextEdit)</div>
        {/if}
      {:else}
        <div class="empty">Search for an app by name, or enter the bundle ID directly.</div>
      {/if}
    </div>
    <div class="modal-actions">
      <button class="button" type="button" on:click={close}>Cancel</button>
      <button class="button primary" type="button" disabled={!canAdd} on:click={submit}>Add</button>
    </div>
  </div>
</div>

<style>
  .modal-drop-hint {
    padding: 6px 12px;
    font-size: 12px;
    color: var(--text-secondary, #888);
    text-align: center;
    border-top: 1px dashed transparent;
    transition: color 120ms ease;
  }

  .modal-drop-hint.dragging {
    color: var(--accent-color, #4f8cff);
    font-weight: 600;
  }
</style>
