<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";

    interface Version {
        id: string;
        version_type: string;
        release_time: string;
        installed: boolean;
    }

    let selectedVersion: string = $state("");
    let currentAccount: string | null = $state(null);
    let installedVersions: Version[] = $state([]);
    let loading: boolean = $state(true);
    let isLaunching: boolean = $state(false);

    onMount(async () => {
        try {
            currentAccount = await invoke<string | null>('accountgetcurrent');
            installedVersions = await invoke<Version[]>('get_installed_versions_info');
            if (installedVersions.length > 0) {
                selectedVersion = installedVersions[0].id;
            }
        } catch (error) {
            console.error('Failed to load data:', error);
        } finally {
            loading = false;
        }
    });

    function launch() {
        if (!selectedVersion) {
            console.error("No version selected");
            return;
        }
        
        isLaunching = true;
        invoke("launchprocess", { version: selectedVersion })
            .then(() => {
                console.log("Process launched successfully");
            })
            .catch((error) => {
                console.error("Error launching process:", error); 
            })
            .finally(() => {
                isLaunching = false;
            });
    }


</script>


<main>
    <div class="flex flex-row gap-7 text-xl p-4 font-roboto font-medium">

        <div>
           <h1 class="text-gray-400">Storage Used:</h1> 
           <h2 class="text-white">0 GB</h2>
        </div>

        <div>
            <h1 class="text-gray-400">Play Time:</h1>
            <h2 class="text-white">0 Hours</h2>
        </div>

        <div>
            <h1 class="text-gray-400">Last Played:</h1>
            <h2 class="text-white">0 Hours ago</h2>
        </div>

        <div>
           <h1 class="text-gray-400">Account Selected:</h1> 
           <h2 class="text-white">{currentAccount || 'None'}</h2>
        </div>

        <div>
           <h1 class="text-gray-400">Version Selected:</h1> 
           {#if loading}
           <h2 class="text-gray-500">Loading...</h2>
           {:else if installedVersions.length === 0}
           <h2 class="text-gray-500">No versions installed</h2>
           {:else}
           <select bind:value={selectedVersion} class="w-full bg-neutral-800 text-white font-rubik outline-none ring-0 focus:ring-0 focus:outline-none border-0">
             {#each installedVersions as version}
             <option value={version.id}>{version.id}</option>
             {/each}
           </select>
           {/if}
        </div>

    </div>

    <div class="flex flex-row gap-3 p-4">
        <button 
            onclick={launch}
            disabled={!selectedVersion || loading || isLaunching}
            class="text-white text-xl font-roboto font-medium py-5 px-15 bg-green-400 rounded-2xl transition-all ease-in duration-300 hover:bg-green-500 hover:shadow-green-900 shadow-lg active:bg-green-900 cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed" 
        >
            {isLaunching ? 'Launching...' : 'Launch'}
        </button>
    </div>
</main>