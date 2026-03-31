<script lang="ts">
  import '../app.css';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import UpdateNotifier from '$lib/components/UpdateNotifier.svelte';
  import { setupTray } from '$lib/traySetup';
  import { onMount } from 'svelte';

  let { children } = $props();

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

  onMount(async () => {
    try {
      console.log('📱 Initializing system tray...');
      await setupTray();
      console.log('✅ Tray setup complete');
    } catch (err) {
      console.error('❌ Failed to setup tray:', err);
    }
  });

</script>

<main class="flex h-screen w-full">
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
</main>