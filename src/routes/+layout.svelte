<script lang="ts">
  import '../app.css';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import UpdateNotifier from '$lib/components/UpdateNotifier.svelte';
  import JavaBootstrap from '$lib/components/JavaBootstrap.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let { children } = $props();
  let needsBootstrap: boolean = $state(false);
  let bootstrapComponents: string[] | null = $state(null);
  let bootstrapReady: boolean = $state(false);

  function goToAccount(){
    goto('/account')
  }

  function goToHome() {
    goto('/')
  }

  function goToLibrary(){
    goto('/library')
  }

  function goToSettings(){
    goto('/settings')
  }

  const isActive = (path: string) => {
    return $page.url.pathname === path || $page.url.pathname.startsWith(path + '/');
  };

  // Disable Ctrl+R refresh globally
  function handleDisableRefresh(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'r') {
      e.preventDefault();
    }
  }

  onMount(async () => {
    window.addEventListener('keydown', handleDisableRefresh);

    try {
      const javaStatus = await invoke<string>('check_java_status');
      console.log('[Layout] Java status:', javaStatus);
      
      if (javaStatus.startsWith('bootstrap_needed')) {
        // Extract component list if provided
        const parts = javaStatus.split(':');
        if (parts.length > 1) {
          bootstrapComponents = parts[1].split(',');
        }
        needsBootstrap = true;
      } else {
        bootstrapReady = true;
      }
    } catch (err) {
      console.error('[Layout] Java check failed:', err);
      // Default to ready if check fails
      bootstrapReady = true;
    }
    
    // Clean up corrupted versions on startup
    try {
      const cleaned = await invoke<string[]>('clean_corrupted_versions');
      if (cleaned.length > 0) {
        console.log('[Layout] Cleaned corrupted version directories:', cleaned);
      }
    } catch (err) {
      console.warn('[Layout] Failed to clean corrupted versions:', err);
    }
  });

  function handleBootstrapComplete() {
    console.log('[Layout] Bootstrap complete, showing main UI');
    needsBootstrap = false;
    bootstrapReady = true;
  }
</script>

<main class="flex h-screen w-full">
  {#if needsBootstrap}
    <JavaBootstrap onComplete={handleBootstrapComplete} components={bootstrapComponents} />
  {:else if bootstrapReady}
    <!-- NavBar -->
    <div class="text-center bg-neutral-800 px-5 py-5 flex flex-col w-30 gap-8 shrink-0">
      <div class="font-roboto font-bold text-2xl antialiased text-green-400 text-shadow-lg/30 transition-all hover:animate-ping">F</div>

      <button 
        onclick={goToHome} 
        class={`text-gray-500 transition-all duration-300 ease-in-out hover:text-green-400 hover:drop-shadow-green-300 drop-shadow-2xl ${
          isActive('/') && !isActive('/library') ? 'text-green-400' : ''
        }`}
      >
        <i class="fi-rr-home text-xl text-shadow-lg/30"></i>
        <p class="font-rubik text-shadow-lg/30">Home</p>
      </button>

      <button 
        type="button" 
        onclick={goToLibrary} 
        class={`text-gray-500 transition-colors ease-in-out hover:text-green-400 block hover:drop-shadow-green-300 drop-shadow-2xl ${
          isActive('/library') ? 'text-green-400' : ''
        }`}
      >
        <i class="fi fi-rr-books-medical text-xl text-shadow-lg/30"></i>
        <p class="font-rubik text-shadow-lg/30">Library</p>
      </button>

      <button type="button" disabled class="text-gray-600 opacity-50 cursor-not-allowed transition-colors ease-in-out drop-shadow-2xl">
        <i class="fi fi-rr-document-gavel text-xl text-shadow-lg/30"></i>
        <p class="font-rubik text-shadow-lg/30">Mods</p>
      </button>

      <button 
        type="button" 
        onclick={goToAccount} 
        class={`text-gray-500 transition-colors ease-in-out hover:text-green-400 block hover:drop-shadow-green-300 drop-shadow-2xl ${
          isActive('/account') ? 'text-green-400' : ''
        }`}
      >
        <i class="fi fi-rr-user-add text-xl text-shadow-lg/30"></i>
        <p class="font-rubik text-shadow-lg/30">Accounts</p>
      </button>

      <button 
        onclick={goToSettings} 
        type="button" 
        class={`text-gray-500 mt-auto transition-colors ease-in-out hover:text-green-400 hover:drop-shadow-green-300 drop-shadow-2xl ${
          isActive('/settings') ? 'text-green-400' : ''
        }`}
      >
        <i class="fi fi-rr-settings-sliders text-xl text-shadow-lg/30"></i>
        <p class="font-rubik text-shadow-lg/30">Settings</p>
      </button>
    </div>

    <!-- Page content -->
    <div class="flex-1 h-full bg-neutral-900 overflow-y-auto [&::-webkit-scrollbar]:hidden">
      {@render children()}
    </div>

    <!-- Update Notifier -->
    <UpdateNotifier />
  {/if}
</main>