<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";

    interface Version {
        id: string;
        version_type: string;
        release_time: string;
        installed: boolean;
    }

    interface GameProfile {
        name: string;
        base_version: string;
        modloader: string;
        modloader_version: string | null;
        created_date: string;
        last_played: string | null;
        ram_mb: number;
        enabled_mods: string[];
    }

    let selectedProfile: string = $state("");
    let selectedVersion: string = $state("");
    let currentAccount: string | null = $state(null);
    let installedVersions: Version[] = $state([]);
    let gameProfiles: GameProfile[] = $state([]);
    let lastPlayedProfile: GameProfile | null = $state(null);
    let loading: boolean = $state(true);
    let isLaunching: boolean = $state(false);
    let activeTab: "vanilla" | "profiles" = $state("profiles");

    onMount(async () => {
        try {
            currentAccount = await invoke<string | null>('accountgetcurrent');
            installedVersions = await invoke<Version[]>('get_installed_versions_info');
            gameProfiles = await invoke<GameProfile[]>('get_all_profiles');
            
            // Find the last played profile
            lastPlayedProfile = gameProfiles.reduce((latest, current) => {
                if (!latest) return current;
                if (!current.last_played) return latest;
                if (!latest.last_played) return current;
                return new Date(current.last_played) > new Date(latest.last_played) ? current : latest;
            }, null as GameProfile | null);
            
            if (gameProfiles.length > 0) {
                selectedProfile = gameProfiles[0].name;
            } else if (installedVersions.length > 0) {
                selectedVersion = installedVersions[0].id;
            }
        } catch (error) {
            console.error('Failed to load data:', error);
        } finally {
            loading = false;
        }
    });

    function switchToProfiles() {
        activeTab = 'profiles';
    }

    function switchToVanilla() {
        activeTab = 'vanilla';
    }

    function launch() {
        if (activeTab == "profiles" && !selectedProfile) {
            console.error("No profile selected");
            return;
        }
        if (activeTab == "vanilla" && !selectedVersion) {
            console.error("No version selected");
            return;
        }
        
        isLaunching = true;
        
        const launchPromise = activeTab == "profiles"
            ? invoke("launchprocess", { profileName: selectedProfile })
            : invoke("launchprocess", { version: selectedVersion });

        launchPromise
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
    <div class="flex flex-row gap-7 text-xl p-4 font-roboto font-medium border-b border-neutral-700">
        <div>
           <h1 class="text-gray-400">Account:</h1> 
           <h2 class="text-white">{currentAccount || 'None'}</h2>
        </div>

        <div>
            <h1 class="text-gray-400">Play Time:</h1>
            <h2 class="text-white">0 Hours</h2>
        </div>

        <div>
            <h1 class="text-gray-400">Last Played:</h1>
            <h2 class="text-white">{lastPlayedProfile ? lastPlayedProfile.name : 'None'}</h2>
        </div>
    </div>

    <div class="flex flex-col gap-4 p-6">
        <!-- Tab Selection -->
        <div class="flex gap-2">
            <button 
                onclick={switchToProfiles}
                class="px-6 py-2 rounded-lg font-medium transition-all {activeTab == 'profiles' ? 'bg-green-400 text-neutral-900' : 'bg-neutral-700 text-white hover:bg-neutral-600'}"
            >
                Modded Profiles
            </button>
            <button 
                onclick={switchToVanilla}
                class="px-6 py-2 rounded-lg font-medium transition-all {activeTab == 'vanilla' ? 'bg-green-400 text-neutral-900' : 'bg-neutral-700 text-white hover:bg-neutral-600'}"
            >
                Vanilla Versions
            </button>
        </div>

        <!-- Profiles Tab -->
        {#if activeTab == 'profiles'}
        <div class="flex flex-col gap-3">
            <div class="flex flex-col gap-1">
                <label for="select-profile" class="text-green-400 text-sm font-bold">Select Profile</label>
                {#if loading}
                <div class="text-gray-500">Loading profiles...</div>
                {:else if gameProfiles.length === 0}
                <div class="text-gray-500 py-4 text-center">No profiles created yet</div>
                {:else}
                <select id="select-profile" bind:value={selectedProfile} class="bg-neutral-800 text-white py-2 px-3 rounded-lg focus:outline-none focus:ring-2 focus:ring-green-400">
                    {#each gameProfiles as profile}
                    <option value={profile.name}>
                        {profile.name} ({profile.base_version} - {profile.modloader})
                    </option>
                    {/each}
                </select>
                {/if}
            </div>

            {#if selectedProfile && gameProfiles.length > 0}
            {@const profile = gameProfiles.find(p => p.name === selectedProfile)}
            {#if profile}
            <div class="bg-neutral-800 rounded-lg p-4 text-sm">
                <div class="grid grid-cols-2 gap-2 text-gray-300">
                    <div>Base: {profile.base_version}</div>
                    <!-- <div>Loader: {profile.modloader}</div> -->
                    <div>RAM: {profile.ram_mb}MB</div>
                    <!-- <div>Mods: {profile.enabled_mods.length}</div> -->
                </div>
            </div>
            {/if}
            {/if}
        </div>
        {/if}

        <!-- Vanilla Tab -->
        {#if activeTab == 'vanilla'}
        <div class="flex flex-col gap-3">
            <div class="flex flex-col gap-1">
                <label for="select-version" class="text-green-400 text-sm font-bold">Select Version</label>
                {#if loading}
                <div class="text-gray-500">Loading versions...</div>
                {:else if installedVersions.length === 0}
                <div class="text-gray-500 py-4 text-center">No versions installed</div>
                {:else}
                <select id="select-version" bind:value={selectedVersion} class="bg-neutral-800 text-white py-2 px-3 rounded-lg focus:outline-none focus:ring-2 focus:ring-green-400">
                    {#each installedVersions as version}
                    <option value={version.id}>{version.id}</option>
                    {/each}
                </select>
                {/if}
            </div>
        </div>
        {/if}

        <!-- Launch Button -->
        <div class="flex gap-3 pt-4">
            <button 
                onclick={launch}
                disabled={
                    (activeTab == 'profiles' && !selectedProfile) || 
                    (activeTab == 'vanilla' && !selectedVersion) || 
                    loading || isLaunching
                }
                class="flex-1 text-white text-lg font-roboto font-bold py-4 px-6 bg-green-400 rounded-xl transition-all hover:bg-green-500 active:bg-green-600 shadow-lg disabled:opacity-50 disabled:cursor-not-allowed" 
            >
                {isLaunching ? 'Launching...' : 'PLAY'}
            </button>
            <a href="/library" class="px-6 py-4 bg-blue-600 text-white rounded-xl font-bold hover:bg-blue-700 transition-all flex items-center">
                Manage
            </a>
        </div>
    </div>
</main>