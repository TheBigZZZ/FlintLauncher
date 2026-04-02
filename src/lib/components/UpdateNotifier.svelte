<script lang="ts">
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { invoke } from '@tauri-apps/api/core';

	interface UpdateInfo {
		current_version: string;
		latest_version: string;
		release_notes: string | null;
		update_available: boolean;
	}

	let showUpdateDialog = $state(false);
	let updateInfo = $state<UpdateInfo | null>(null);
	let isDownloading = $state(false);
	let downloadStatus = $state('');

	onMount(() => {
		if (browser) {
			checkForUpdates();
		}
	});

	async function checkForUpdates() {
		if (!browser) return;

		try {
			const version = '0.2.0'; // Should match tauri.conf.json version
			console.log('Checking for updates... current version:', version);
			const result = await invoke<UpdateInfo>('check_for_updates', { currentVersion: version });

			console.log('Update check result:', result);
			if (result.update_available) {
				updateInfo = result;
				showUpdateDialog = true;
				console.log('Update available! Showing dialog.');
			} else {
				console.log('No update available. Current:', result.current_version, 'Latest:', result.latest_version);
				if (result.release_notes) {
					console.log('Info:', result.release_notes);
				}
			}
		} catch (error) {
			console.error('Failed to check for updates:', error);
		}
	}

	async function handleDownloadAndInstall() {
		if (!updateInfo || !browser) return;

		isDownloading = true;
		downloadStatus = 'Downloading update...';

		try {
			// For now, just show instructions to download from GitHub
			// In the future, this could handle automated downloads
			downloadStatus = 'Please download from: https://github.com/FaizeenHoque/FlintLauncher/releases';
			setTimeout(() => {
				isDownloading = false;
				closeDialog();
			}, 3000);
		} catch (error) {
			downloadStatus = `Error: ${error}`;
			isDownloading = false;
		}
	}

	function closeDialog() {
		showUpdateDialog = false;
		updateInfo = null;
	}
</script>

{#if showUpdateDialog && updateInfo}
	<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
		<div class="bg-gray-900 rounded-lg p-6 max-w-md w-full mx-4 border border-green-400">
			<h2 class="text-xl font-bold text-white mb-4">Update Available</h2>

			<div class="space-y-4 text-gray-300">
				<div>
					<p class="text-sm opacity-75">Current Version</p>
					<p class="font-mono text-white">{updateInfo.current_version}</p>
				</div>

				<div>
					<p class="text-sm opacity-75">Latest Version</p>
					<p class="font-mono text-green-400">{updateInfo.latest_version}</p>
				</div>

				{#if updateInfo.release_name}
					<div>
						<p class="text-sm opacity-75">Release</p>
						<p class="text-white">{updateInfo.release_name}</p>
					</div>
				{/if}

				{#if updateInfo.release_notes}
					<div>
						<p class="text-sm opacity-75">Release Notes</p>
						<p class="text-white text-sm max-h-32 overflow-y-auto">
							{updateInfo.release_notes}
						</p>
					</div>
				{/if}

				{#if downloadStatus}
					<p class="text-sm text-yellow-400">{downloadStatus}</p>
				{/if}
			</div>

			<div class="flex gap-3 mt-6">
				<button
					onclick={closeDialog}
					disabled={isDownloading}
					class="flex-1 px-4 py-2 bg-gray-800 text-white rounded hover:bg-gray-700 disabled:opacity-50"
				>
					Later
				</button>
				<button
					onclick={handleDownloadAndInstall}
					disabled={isDownloading}
					class="flex-1 px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50"
				>
					{isDownloading ? 'Downloading...' : 'Update Now'}
				</button>
			</div>
		</div>
	</div>
{/if}
