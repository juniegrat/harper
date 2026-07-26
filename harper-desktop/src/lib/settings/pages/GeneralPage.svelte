<script lang="ts">
import type { Dialect } from 'harper.js';
import { onMount } from 'svelte';
import { Client } from '$lib/client';
import { DesktopUpdater } from '$lib/DesktopUpdater';
import { DIALECT_OPTIONS } from '../settings-data';

const DialectValue = {
	American: 0,
	British: 1,
	Australian: 2,
	Canadian: 3,
	Indian: 4,
} as const;

let menuBar = true;
let launchAtStartup = false;
let autoUpdate = true;
let dialect = 'american';
let isDialectLoading = true;
let isDialectSaving = false;
let dialectError = '';
let french = false;
let isFrenchLoading = true;
let isFrenchSaving = false;
let frenchError = '';
let isLaunchAtStartupLoading = true;
let isLaunchAtStartupSaving = false;
let launchAtStartupError = '';
let isAutoUpdateLoading = true;
let isAutoUpdateSaving = false;
let isCheckingForUpdates = false;
let autoUpdateError = '';
let updateStatus = '';
let currentVersion = '';
let latestVersion = '';
let debounceMs = 0;
let debounceMsInput = '0';
let isDebounceLoading = true;
let isDebounceSaving = false;
let debounceError = '';

onMount(() => {
	void loadDialect();
	void loadFrench();
	void loadLaunchAtStartup();
	void loadAutoUpdate();
	void loadUpdateVersions();
	void loadDebounceMs();

	const refreshSettings = () => {
		if (!isDialectSaving) {
			void loadDialect();
		}

		if (!isFrenchSaving) {
			void loadFrench();
		}

		if (!isLaunchAtStartupSaving) {
			void loadLaunchAtStartup();
		}

		if (!isAutoUpdateSaving) {
			void loadAutoUpdate();
		}

		if (!isCheckingForUpdates) {
			void loadUpdateVersions();
		}

		if (!isDebounceSaving) {
			void loadDebounceMs();
		}
	};

	window.addEventListener('focus', refreshSettings);

	return () => {
		window.removeEventListener('focus', refreshSettings);
	};
});

async function loadDialect() {
	isDialectLoading = true;
	dialectError = '';

	try {
		dialect = dialectToSettingsValue(await Client.getDialect());
	} catch (error) {
		dialectError = `Unable to load dialect: ${error}`;
	} finally {
		isDialectLoading = false;
	}
}

async function setDialect(value: string) {
	const previousDialect = dialect;

	dialect = value;
	isDialectSaving = true;
	dialectError = '';

	try {
		await Client.setDialect(settingsValueToDialect(value));
	} catch (error) {
		dialect = previousDialect;
		dialectError = `Unable to save dialect: ${error}`;
	} finally {
		isDialectSaving = false;
	}
}

async function loadFrench() {
	isFrenchLoading = true;
	frenchError = '';

	try {
		french = await Client.getFrench();
	} catch (error) {
		frenchError = `Unable to load French setting: ${error}`;
	} finally {
		isFrenchLoading = false;
	}
}

async function setFrench(value: boolean) {
	const previousFrench = french;

	french = value;
	isFrenchSaving = true;
	frenchError = '';

	try {
		await Client.setFrench(value);
	} catch (error) {
		french = previousFrench;
		frenchError = `Unable to save French setting: ${error}`;
	} finally {
		isFrenchSaving = false;
	}
}

async function setLanguage(value: string) {
	// French replaces the whole English pipeline (parser, dictionary, rules),
	// so it is a language choice, not an add-on over the dialect.
	if (value === 'french') {
		await setFrench(true);
		return;
	}

	if (french) {
		await setFrench(false);
	}

	if (value !== dialect) {
		await setDialect(value);
	}
}

async function loadLaunchAtStartup() {
	isLaunchAtStartupLoading = true;
	launchAtStartupError = '';

	try {
		launchAtStartup = await Client.getLaunchAtStartup();
	} catch (error) {
		launchAtStartupError = `Unable to load startup setting: ${error}`;
	} finally {
		isLaunchAtStartupLoading = false;
	}
}

async function setLaunchAtStartup(enabled: boolean) {
	const previousLaunchAtStartup = launchAtStartup;

	launchAtStartup = enabled;
	isLaunchAtStartupSaving = true;
	launchAtStartupError = '';

	try {
		await Client.setLaunchAtStartup(enabled);
	} catch (error) {
		launchAtStartup = previousLaunchAtStartup;
		launchAtStartupError = `Unable to save startup setting: ${error}`;
	} finally {
		isLaunchAtStartupSaving = false;
	}
}

async function loadAutoUpdate() {
	isAutoUpdateLoading = true;
	autoUpdateError = '';

	try {
		autoUpdate = await Client.getAutoUpdate();
	} catch (error) {
		autoUpdateError = `Unable to load update setting: ${error}`;
	} finally {
		isAutoUpdateLoading = false;
	}
}

async function setAutoUpdate(enabled: boolean) {
	const previousAutoUpdate = autoUpdate;

	autoUpdate = enabled;
	isAutoUpdateSaving = true;
	autoUpdateError = '';
	updateStatus = '';

	try {
		await Client.setAutoUpdate(enabled);
	} catch (error) {
		autoUpdate = previousAutoUpdate;
		autoUpdateError = `Unable to save update setting: ${error}`;
	} finally {
		isAutoUpdateSaving = false;
	}
}

async function loadUpdateVersions() {
	try {
		const [current, latest] = await Promise.all([
			DesktopUpdater.getCurrentVersion(),
			DesktopUpdater.getLatestVersion(),
		]);
		currentVersion = current;
		latestVersion = latest;
	} catch (error) {
		console.error('Unable to load update versions.', error);
	}
}

async function checkForUpdates() {
	isCheckingForUpdates = true;
	autoUpdateError = '';
	updateStatus = 'Checking for updates...';

	try {
		await Client.setLastUpdateCheck(Date.now());
		const result = await DesktopUpdater.updateToLatest();
		updateStatus = result.message;

		if (result.latestVersion != null) {
			latestVersion = result.latestVersion;
		}

		currentVersion = result.currentVersion ?? (await DesktopUpdater.getCurrentVersion());
	} catch (error) {
		autoUpdateError = `Unable to check for updates: ${error}`;
		updateStatus = '';
	} finally {
		isCheckingForUpdates = false;
	}
}

async function loadDebounceMs() {
	isDebounceLoading = true;
	debounceError = '';

	try {
		debounceMs = await Client.getDebounceMs();
		debounceMsInput = String(debounceMs);
	} catch (error) {
		debounceError = `Unable to load debounce delay: ${error}`;
	} finally {
		isDebounceLoading = false;
	}
}

async function saveDebounceMs() {
	const parsedDebounceMs = Number(debounceMsInput);

	if (!Number.isInteger(parsedDebounceMs) || parsedDebounceMs < 0) {
		debounceError = 'Debounce delay must be a non-negative whole number.';
		debounceMsInput = String(debounceMs);
		return;
	}

	const previousDebounceMs = debounceMs;
	debounceMs = parsedDebounceMs;
	debounceMsInput = String(parsedDebounceMs);
	isDebounceSaving = true;
	debounceError = '';

	try {
		await Client.setDebounceMs(parsedDebounceMs);
	} catch (error) {
		debounceMs = previousDebounceMs;
		debounceMsInput = String(previousDebounceMs);
		debounceError = `Unable to save debounce delay: ${error}`;
	} finally {
		isDebounceSaving = false;
	}
}

function dialectToSettingsValue(dialect: Dialect): string {
	switch (dialect) {
		case DialectValue.British:
			return 'british';
		case DialectValue.Canadian:
			return 'canadian';
		case DialectValue.Australian:
			return 'australian';
		case DialectValue.Indian:
			return 'indian';
		default:
			return 'american';
	}
}

function settingsValueToDialect(value: string): Dialect {
	switch (value) {
		case 'british':
			return DialectValue.British as Dialect;
		case 'canadian':
			return DialectValue.Canadian as Dialect;
		case 'australian':
			return DialectValue.Australian as Dialect;
		case 'indian':
			return DialectValue.Indian as Dialect;
		default:
			return DialectValue.American as Dialect;
	}
}
</script>

<section>
        <div class="stanza">
          <div class="eyebrow">General</div>
          <div class="rows">
            <div class="row top">
              <div>
                <strong>Keep Harper in the menu bar</strong>
                <p>Shows the Harper icon so you can open settings without opening the main app.</p>
              </div>
              <button
                class:checked={menuBar}
                class="checkbox"
                type="button"
                role="checkbox"
                disabled
                title="Not wired yet"
                aria-checked={menuBar}
              >
                {#if menuBar}<span class="settings-icon icon-check" aria-hidden="true"></span>{/if}
              </button>
            </div>

            <div class="row">
              <div>
                <strong>Launch Harper at startup</strong>
                <p>Harper will start silently when you log in.</p>
              </div>
              <button
                class:checked={launchAtStartup}
                class="checkbox"
                type="button"
                role="checkbox"
                disabled={isLaunchAtStartupLoading || isLaunchAtStartupSaving}
                aria-checked={launchAtStartup}
                on:click={() => setLaunchAtStartup(!launchAtStartup)}
              >
                {#if launchAtStartup}<span class="settings-icon icon-check" aria-hidden="true"></span>{/if}
              </button>
            </div>
            {#if isLaunchAtStartupLoading}
              <p class="result-summary">Loading startup setting...</p>
            {:else if launchAtStartupError}
              <p class="result-summary">{launchAtStartupError}</p>
            {:else if isLaunchAtStartupSaving}
              <p class="result-summary">Saving startup setting...</p>
            {/if}

            <div class="row top">
              <div>
                <strong>Automatically check for updates</strong>
                <p>Harper will check for new versions daily.</p>
                <p class="result-summary">
                  Current version: {currentVersion || 'loading...'} · Latest version: {latestVersion || 'loading...'}
                </p>
              </div>
              <button
                class:checked={autoUpdate}
                class="checkbox"
                type="button"
                role="checkbox"
                disabled={isAutoUpdateLoading || isAutoUpdateSaving}
                aria-checked={autoUpdate}
                on:click={() => setAutoUpdate(!autoUpdate)}
              >
                {#if autoUpdate}<span class="settings-icon icon-check" aria-hidden="true"></span>{/if}
              </button>
            </div>
            <div class="inline-row">
              <button
                class="button"
                type="button"
                disabled={isCheckingForUpdates}
                on:click={checkForUpdates}
              >
                {isCheckingForUpdates ? 'Checking...' : 'Check for updates'}
              </button>
            </div>
            {#if isAutoUpdateLoading}
              <p class="result-summary">Loading update setting...</p>
            {:else if autoUpdateError}
              <p class="result-summary">{autoUpdateError}</p>
            {:else if isAutoUpdateSaving}
              <p class="result-summary">Saving update setting...</p>
            {:else if updateStatus}
              <p class="result-summary">{updateStatus}</p>
            {/if}
          </div>
        </div>

        <div class="divider"></div>

        <div class="stanza">
          <div class="eyebrow">Language</div>
          <p class="section-copy">
            Choose the language Harper uses to check spelling and grammar. Selecting Français
            replaces the English pipeline (parser, dictionary and rules) entirely.
          </p>
          <div class="inline-row">
            <label for="language">Language:</label>
            <select
              id="language"
              class="select wide"
              disabled={isDialectLoading || isDialectSaving || isFrenchLoading || isFrenchSaving}
              value={french ? 'french' : dialect}
              on:change={(event) => setLanguage(event.currentTarget.value)}
            >
              {#each DIALECT_OPTIONS as option}
                <option value={option.value}>{option.label}</option>
              {/each}
              <option value="french">Français (expérimental)</option>
            </select>
          </div>
          {#if isDialectLoading || isFrenchLoading}
            <p class="result-summary">Loading language...</p>
          {:else if dialectError || frenchError}
            <p class="result-summary">{dialectError || frenchError}</p>
          {:else if isDialectSaving || isFrenchSaving}
            <p class="result-summary">Saving language...</p>
          {/if}
        </div>

        <div class="divider"></div>

        <div class="stanza">
          <div class="eyebrow">Writing</div>
          <p class="section-copy">
            Choose how long Harper waits after text changes before checking it. Use 0 ms for
            immediate checking.
          </p>
          <div class="inline-row">
            <label for="debounce-ms">Debounce delay:</label>
            <input
              id="debounce-ms"
              class="select"
              type="number"
              min="0"
              step="50"
              disabled={isDebounceLoading || isDebounceSaving}
              value={debounceMsInput}
              on:input={(event) => (debounceMsInput = event.currentTarget.value)}
              on:change={saveDebounceMs}
            />
            <span>ms</span>
          </div>
          {#if isDebounceLoading}
            <p class="result-summary">Loading debounce delay...</p>
          {:else if debounceError}
            <p class="result-summary">{debounceError}</p>
          {:else if isDebounceSaving}
            <p class="result-summary">Saving debounce delay...</p>
          {/if}
        </div>
      </section>
