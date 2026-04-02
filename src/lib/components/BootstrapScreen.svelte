<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let { onComplete = () => {} } = $props();

  let progress: number = $state(0);
  let logs: string[] = $state([]);
  let currentComponent: string = $state('');
  let totalComponents: number = $state(0);
  let completedComponents: number = $state(0);
  let error: string | null = $state(null);
  let isComplete: boolean = $state(false);
  let isRetrying: boolean = $state(false);
  let showDetails: boolean = $state(false);
  let componentsToDownload: string[] = $state([]);
  let logsContainer: HTMLElement | undefined = $state();

  // Auto-scroll logs to bottom whenever they change
  $effect(() => {
    if (logsContainer && logs.length > 0) {
      requestAnimationFrame(() => {
        if (logsContainer) {
          logsContainer.scrollTop = logsContainer.scrollHeight;
        }
      });
    }
  });

  onMount(async () => {
    try {
      // Listen to bootstrap events
      const unlistenStart = await listen<{ total_components: number }>('bootstrap:start', (event) => {
        totalComponents = event.payload.total_components;
        logs = [`[bootstrap] Starting download of ${totalComponents} Java components`];
      });

      const unlistenProgress = await listen<{
        component: string;
        current_file: string;
        downloaded: number;
        total: number;
      }>('bootstrap:progress', (event) => {
        currentComponent = event.payload.component;
        const percent = totalComponents > 0 
          ? Math.round(((completedComponents + (event.payload.downloaded / event.payload.total)) / totalComponents) * 100)
          : 0;
        progress = Math.min(percent, 99);
        
        logs = [...logs, `[${event.payload.component}] ${event.payload.downloaded}/${event.payload.total} - ${event.payload.current_file}`];
        
        // Auto-scroll
        setTimeout(() => {
          const container = document.querySelector('[data-logs]');
          if (container) container.scrollTop = container.scrollHeight;
        }, 0);
      });

      const unlistenComponentDone = await listen<{ component: string }>('bootstrap:component_done', (event) => {
        completedComponents++;
        progress = Math.round((completedComponents / totalComponents) * 100);
        logs = [...logs, `[${event.payload.component}] Done`];
      });

      const unlistenDone = await listen('bootstrap:done', () => {
        progress = 100;
        logs = [...logs, '[bootstrap] All components installed successfully'];
        isComplete = true;
        setTimeout(() => onComplete(), 1500);
      });

      const unlistenError = await listen<{ message: string }>('bootstrap:error', (event) => {
        error = event.payload.message;
        logs = [...logs, `[ERROR] ${event.payload.message}`];
      });

      // Start bootstrap
      await invoke('bootstrap_java_runtimes');
    } catch (err) {
      error = `Failed to start Java bootstrap: ${err}`;
      console.error('Bootstrap error:', err);
    }
  });

  async function retry() {
    isRetrying = true;
    error = null;
    progress = 0;
    logs = ['[bootstrap] Retrying...'];
    completedComponents = 0;
    
    try {
      await invoke('bootstrap_java_runtimes', { components: componentsToDownload.length > 0 ? componentsToDownload : null });
      isRetrying = false;
    } catch (err) {
      error = `Retry failed: ${err}`;
      isRetrying = false;
    }
  }
</script>

<div class="fixed inset-0 bg-linear-to-br from-neutral-900 via-neutral-800 to-neutral-900 flex items-center justify-center p-4 z-50 overflow-hidden">
  <div class="w-full max-w-md flex flex-col h-screen max-h-screen box-border">
    <!-- Logo/Wordmark -->
    <div class="text-center shrink-0">
      <div class="text-7xl font-bold text-green-400 font-roboto tracking-wider drop-shadow-2xl mb-4">F</div>
      <p class="text-gray-300 text-lg font-medium">Flint Launcher</p>
      <p class="text-gray-500 text-sm mt-2">Initializing Java Runtime Environment</p>
    </div>

    <!-- Title -->
    <h2 class="text-3xl font-bold text-center my-8 text-white shrink-0">Installing Java Runtime</h2>

    <!-- Scrollable Content Area -->
    <div class="flex-1 overflow-y-auto min-h-0 flex flex-col gap-6 px-2">
      {#if error}
        <!-- Error State -->
        <div class="bg-red-900 bg-opacity-30 border border-red-500 border-opacity-50 rounded-lg p-6 shrink-0">
          <p class="text-red-300 text-sm mb-4">{error}</p>
          <button
            onclick={retry}
            disabled={isRetrying}
            class="w-full bg-red-600 hover:bg-red-700 disabled:bg-red-800 disabled:opacity-50 text-white py-3 rounded-lg font-medium transition-colors"
          >
            {isRetrying ? 'Retrying...' : 'Retry'}
          </button>
        </div>
      {/if}

      {#if !isComplete && !error}
        <!-- Progress Section -->
        <div class="bg-neutral-800 bg-opacity-50 rounded-lg p-6 border border-neutral-700 shrink-0">
          <!-- Progress Bar -->
          <div class="w-full bg-neutral-700 bg-opacity-50 rounded-full h-2 mb-4 overflow-hidden">
            <div
              class="bg-linear-to-r from-green-500 to-green-400 h-full transition-all duration-300 shadow-lg shadow-green-500/50"
              style={`width: ${progress}%`}
            ></div>
          </div>

          <!-- Percentage -->
          <div class="flex justify-between items-center mb-6">
            <span class="text-gray-300 text-sm font-medium">
              {completedComponents}/{totalComponents} components installed
            </span>
            <span class="text-green-400 font-bold text-lg">{progress}%</span>
          </div>

          <!-- Current Component -->
          {#if currentComponent}
            <p class="text-gray-200 text-sm text-center font-medium">Downloading {currentComponent}...</p>
          {:else if totalComponents > 0}
            <p class="text-gray-300 text-sm text-center">Preparing installation...</p>
          {/if}
        </div>
      {/if}

      {#if isComplete}
        <!-- Success State -->
        <div class="bg-green-900 bg-opacity-30 border border-green-500 border-opacity-50 rounded-lg p-6 text-center shrink-0">
          <p class="text-green-300 text-base font-medium">✓ Java installation complete!</p>
        </div>
      {/if}

      <!-- Details Toggle -->
      <details bind:open={showDetails} class="bg-neutral-800 bg-opacity-30 border border-neutral-700 rounded-lg shrink-0">
        <summary class="cursor-pointer text-gray-300 text-sm font-medium hover:text-green-400 transition-colors flex items-center gap-2 p-4">
          <span class="inline-block transition-transform" style={`transform: rotate(${showDetails ? 180 : 0}deg)`}>
            ▼
          </span>
          Details Log
        </summary>

        <!-- Log Output -->
        <div
          bind:this={logsContainer}
          class="bg-neutral-900 border-t border-neutral-700 rounded-b-lg p-4 max-h-40 overflow-y-auto text-xs font-mono text-gray-300 space-y-1"
        >
          {#each logs as log, i (i)}
            <div class="text-gray-400 wrap-break-word">
              {log}
            </div>
          {/each}
          {#if logs.length === 0}
            <div class="text-gray-600">Initializing...</div>
          {/if}
        </div>
      </details>

      <!-- Footer Message -->
      {#if isComplete}
        <p class="text-center text-gray-400 text-sm shrink-0">Launching Flint Launcher...</p>
      {/if}
    </div>
  </div>
</div>

<style>
  details summary::-webkit-details-marker {
    display: none;
  }
</style>
