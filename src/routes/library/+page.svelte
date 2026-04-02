<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { listen } from '@tauri-apps/api/event';
    import { downloadStore, setInstallingVersion, setInstallProgress, setInstallStatus, setDownloadLogs, addDownloadLog, setError, clearDownloadState } from '$lib/stores/downloadStore';

    interface Version {
        id: string;
        version_type: string;
        url?: string;
        time?: string;
        release_time: string;
        sha1?: string;
        complianceLevel?: string;
        installed: boolean;
    }

    interface GameProfile {
        name: string;
        base_version: string;
        created_date: string;
        last_played: string | null;
        ram_mb: number;
    }

    let versions: Version[] = $state([]);
    let profiles: GameProfile[] = $state([]);
    let selectedVersion: string = $state('');
    let dropdownOpen = $state(false);
    let loading = $state(true);
    let searchQuery: string = $state('');
    let versionPage = $state(0);
    let versionsPerPage = 30;
    let showProfileModal = $state(false);
    let profileName = $state('');
    let profileSelectedVersion = $state('');
    let profileDropdownOpen = $state(false);
    let profileSearchQuery = $state('');
    let creatingProfile = $state(false);
    let currentTab: string = $state('versions');
    let showProfileSettings = $state(false);
    let settingsProfileName = $state('');
    let settingsRamMb = $state(2048);
    
    // Modloader state
    let selectedModloader: 'none' | 'fabric' | 'forge' = $state('none');
    let fabricVersions: any[] = $state([]);
    let forgeVersions: any[] = $state([]);
    let selectedModloaderVersion = $state('');
    let loadingModLoaders = $state(false);
    let modloaderDropdownOpen = $state(false);

    // Activity log autoscroll
    let logContainer: HTMLDivElement;

    // Subscribe to download store
    let installingVersion = $state<string | null>(null);
    let installProgress = $state(0);
    let installStatus = $state('');
    let error = $state('');
    let downloadLogs = $state<string[]>([]);

    // Flag to track if download-progress listener has been set up
    let downloadProgressListenerSetUp = false;

    // Maintain store subscriptions
    $effect(() => {
        const unsubscribe = downloadStore.subscribe(state => {
            installingVersion = state.installingVersion;
            installProgress = state.installProgress;
            installStatus = state.installStatus;
            error = state.error;
            downloadLogs = state.downloadLogs;
        });
        return unsubscribe;
    });

    // Refresh versions periodically while downloading
    $effect(() => {
        if (!installingVersion) return;

        const interval = setInterval(async () => {
            try {
                versions = await invoke('fetch_available_versions');
            } catch (err) {
                console.error('Failed to refresh versions:', err);
            }
        }, 2000); // Refresh every 2 seconds

        return () => clearInterval(interval);
    });

    // Auto-scroll activity log to bottom
    $effect(() => {
        if (logContainer && downloadLogs.length > 0) {
            setTimeout(() => {
                logContainer.scrollTop = logContainer.scrollHeight;
            }, 0);
        }
    });

    // Listen to download progress events - set up once to avoid disconnecting during state updates
    async function setupDownloadProgressListener() {
        if (downloadProgressListenerSetUp) return;
        downloadProgressListenerSetUp = true;

        try {
            await listen('download-progress', (event: any) => {
                const data = event.payload;
                if (data.status === 'completed') {
                    addDownloadLog(`  ✓ ${data.filename} (${data.current}/${data.total})`);
                } else if (data.status === 'failed') {
                    addDownloadLog(`  ✗ ${data.filename} - ${data.error}`);
                } else if (data.status === 'task-error') {
                    addDownloadLog(`  ✗ Task error: ${data.error}`);
                }
            });
        } catch (err) {
            console.error('Failed to listen for download events:', err);
        }
    }

    // Reactive computed versions
    let organizedVersions = $derived.by(() => {
        return organizeVersions(versions, searchQuery);
    });

    async function onMount() {
        // Load versions even if download is in progress
        // This ensures installed versions list is accurate when returning to the page
        await loadVersions();
        await loadProfiles();
        // Set up the download progress listener once
        await setupDownloadProgressListener();
    }

    async function loadVersions() {
        try {
            loading = true;
            // Don't reset logs if currently downloading
            if (!installingVersion) {
                setDownloadLogs(['Loading versions from Mojang manifest...']);
            } else {
                addDownloadLog('Refreshing version list...');
            }
            versions = await invoke('fetch_available_versions');
            addDownloadLog(`[OK] Loaded ${versions.length} versions`);
            
            // Sort versions properly handling all formats
            versions = versions.sort((a, b) => compareVersions(a.id, b.id));
            loading = false;
        } catch (err) {
            setError(`Failed to load versions: ${err}`);
            addDownloadLog(`[ERROR] Error: ${err}`);
            loading = false;
        }
    }

    async function loadProfiles() {
        try {
            profiles = await invoke('get_all_profiles');
        } catch (err) {
            console.error('Failed to load profiles:', err);
        }
    }

    // Load modloader versions when a version is selected
    async function loadModLoaderVersions(minecraftVersion: string) {
        if (!minecraftVersion) return;
        
        loadingModLoaders = true;
        selectedModloader = 'none';
        selectedModloaderVersion = '';
        addDownloadLog(`[INFO] Loading modloaders for ${minecraftVersion}...`);
        
        try {
            // Fetch Fabric versions
            console.log(`[DEBUG] Fetching Fabric versions for ${minecraftVersion}`);
            fabricVersions = await invoke('get_fabric_versions', { minecraftVersion: minecraftVersion });
            if (Array.isArray(fabricVersions)) {
                console.log(`[DEBUG] Got ${fabricVersions.length} Fabric versions:`, fabricVersions);
                addDownloadLog(`[OK] Loaded ${fabricVersions.length} Fabric versions for ${minecraftVersion}`);
            } else {
                console.error('[DEBUG] fabricVersions is not an array:', typeof fabricVersions, fabricVersions);
                fabricVersions = [];
                addDownloadLog(`[ERROR] Unexpected Fabric response type`);
            }
        } catch (err) {
            console.error('Failed to load Fabric versions:', err);
            addDownloadLog(`[ERROR] Could not load Fabric versions: ${err}`);
            fabricVersions = [];
        }
        
        try {
            // Fetch Forge versions
            console.log(`[DEBUG] Fetching Forge versions for ${minecraftVersion}`);
            forgeVersions = await invoke('get_forge_versions', { minecraftVersion: minecraftVersion });
            if (Array.isArray(forgeVersions)) {
                console.log(`[DEBUG] Got ${forgeVersions.length} Forge versions:`, forgeVersions);
                addDownloadLog(`[OK] Loaded ${forgeVersions.length} Forge versions for ${minecraftVersion}`);
            } else {
                console.error('[DEBUG] forgeVersions is not an array:', typeof forgeVersions, forgeVersions);
                forgeVersions = [];
                addDownloadLog(`[ERROR] Unexpected Forge response type`);
            }
        } catch (err) {
            console.error('Failed to load Forge versions:', err);
            addDownloadLog(`[ERROR] Could not load Forge versions: ${err}`);
            forgeVersions = [];
        }
        
        loadingModLoaders = false;
        addDownloadLog(`[INFO] Modloader loading complete - Fabric: ${fabricVersions.length}, Forge: ${forgeVersions.length}`);
    }

    async function createProfileHandler() {
        if (!profileName.trim()) {
            setError('Profile name is required');
            return;
        }
        if (!profileSelectedVersion) {
            setError('Minecraft version is required');
            return;
        }

        let effectiveVersion = profileSelectedVersion;

        if (selectedModloader === 'fabric' && selectedModloaderVersion) {
            effectiveVersion = `fabric-loader-${selectedModloaderVersion}-${profileSelectedVersion}`;
        } 

        creatingProfile = true;
        try {
            // Check if version is installed, if not download it first
            const versionInstalled = await invoke('is_version_installed', { version: profileSelectedVersion });
            
            if (!versionInstalled) {
                addDownloadLog(`[INFO] Downloading Minecraft ${profileSelectedVersion}...`);
                await invoke('install_version', { version: profileSelectedVersion });
                addDownloadLog(`[OK] Minecraft ${profileSelectedVersion} downloaded`);
            }
            
            // Install modloader if selected
            if (selectedModloader !== 'none' && selectedModloaderVersion) {
                addDownloadLog(`[INFO] Installing ${selectedModloader} ${selectedModloaderVersion}...`);
                
                try {
                    if (selectedModloader === 'fabric') {
                        await invoke('install_fabric_version', {
                            minecraftVersion: profileSelectedVersion,
                            fabricVersion: selectedModloaderVersion
                        });
                    } else if (selectedModloader === 'forge') {
                        await invoke('install_forge_version', {
                            minecraftVersion: profileSelectedVersion,
                            forgeVersion: selectedModloaderVersion
                        });
                    }
                    
                    addDownloadLog(`[OK] ${selectedModloader} ${selectedModloaderVersion} installed`);
                } catch (err) {
                    addDownloadLog(`[WARN] Modloader installation failed, continuing anyway: ${err}`);
                    console.warn('Modloader installation error:', err);
                }
            }
            
            addDownloadLog(`[INFO] Creating profile '${profileName}'...`);
            await invoke('create_profile', {
                name: profileName,
                baseVersion: effectiveVersion
            });
            
            addDownloadLog(`[OK] Profile '${profileName}' created successfully`);
            await loadProfiles();
            
            // Reset modal
            showProfileModal = false;
            profileName = '';
            profileSelectedVersion = '';
            selectedModloader = 'none';``
            selectedModloaderVersion = '';
            profileSearchQuery = '';
            profileDropdownOpen = false;
            modloaderDropdownOpen = false;
        } catch (err) {
            setError(`Failed to create profile: ${err}`);
            addDownloadLog(`[ERROR] Failed to create profile: ${err}`);
        } finally {
            creatingProfile = false;
        }
    }


    async function deleteProfileHandler(name: string) {
        if (!confirm(`Delete profile '${name}'?`)) return;

        try {
            await invoke('delete_profile', { name });
            addDownloadLog(`[OK] Profile '${name}' deleted`);
            await loadProfiles();
        } catch (err) {
            setError(`Failed to delete profile: ${err}`);
            addDownloadLog(`[ERROR] Failed to delete: ${err}`);
        }
    }

    async function saveRamSettings() {
        if (!settingsProfileName) return;
        
        try {
            await invoke('update_profile_ram', { name: settingsProfileName, ramMb: settingsRamMb });
            addDownloadLog(`[OK] RAM for profile '${settingsProfileName}' updated to ${settingsRamMb}MB`);
            showProfileSettings = false;
            await loadProfiles();
        } catch (err) {
            setError(`Failed to update RAM: ${err}`);
            addDownloadLog(`[ERROR] Failed to update RAM: ${err}`);
        }
    }

    function openProfileSettings(profile: GameProfile) {
        settingsProfileName = profile.name;
        settingsRamMb = profile.ram_mb;
        showProfileSettings = true;
    }

    // Check if version supports Fabric (1.19+)
    function supportsFabric(version: string): boolean {
        const match = version.match(/^(\d+)\.(\d+)/);
        if (!match) return false;
        const major = parseInt(match[1]);
        const minor = parseInt(match[2]);
        return major > 1 || (major === 1 && minor >= 19);
    }

    // Compare and sort version strings properly
    function compareVersions(a: string, b: string): number {
        // Handle release versions (1.20.1, 1.19.2, etc.)
        const releasePattern = /^(\d+)\.(\d+)(?:\.(\d+))?/;
        const aMatch = a.match(releasePattern);
        const bMatch = b.match(releasePattern);
        
        if (aMatch && bMatch) {
            const aMajor = parseInt(aMatch[1]);
            const aMinor = parseInt(aMatch[2]);
            const aPatch = parseInt(aMatch[3] || '0');
            
            const bMajor = parseInt(bMatch[1]);
            const bMinor = parseInt(bMatch[2]);
            const bPatch = parseInt(bMatch[3] || '0');
            
            if (aMajor !== bMajor) return bMajor - aMajor;
            if (aMinor !== bMinor) return bMinor - aMinor;
            return bPatch - aPatch;
        }
        
        // Handle snapshots (23w46a, 23w45b, etc.)
        const snapshotPattern = /^(\d{2})w(\d{2})([a-z])?/;
        const aSnap = a.match(snapshotPattern);
        const bSnap = b.match(snapshotPattern);
        
        if (aSnap && bSnap) {
            const aYear = parseInt(aSnap[1]);
            const aWeek = parseInt(aSnap[2]);
            const aChar = (aSnap[3] || 'z').charCodeAt(0);
            
            const bYear = parseInt(bSnap[1]);
            const bWeek = parseInt(bSnap[2]);
            const bChar = (bSnap[3] || 'z').charCodeAt(0);
            
            if (aYear !== bYear) return bYear - aYear;
            if (aWeek !== bWeek) return bWeek - aWeek;
            return bChar - aChar;
        }
        
        // Handle old versions (b1.7.3, a1.0.3_01, etc.)
        const oldPattern = /^([ab])(\d+)\.(\d+)(?:\.(\d+))?/;
        const aOld = a.match(oldPattern);
        const bOld = b.match(oldPattern);
        
        if (aOld && bOld) {
            const aType = aOld[1].charCodeAt(0);
            const aMajor = parseInt(aOld[2]);
            const aMinor = parseInt(aOld[3]);
            const aPatch = parseInt(aOld[4] || '0');
            
            const bType = bOld[1].charCodeAt(0);
            const bMajor = parseInt(bOld[2]);
            const bMinor = parseInt(bOld[3]);
            const bPatch = parseInt(bOld[4] || '0');
            
            if (aType !== bType) return bType - aType;
            if (aMajor !== bMajor) return bMajor - aMajor;
            if (aMinor !== bMinor) return bMinor - aMinor;
            return bPatch - aPatch;
        }
        
        // Fallback to reverse string comparison
        return b.localeCompare(a);
    }

    // Organize versions by type
    function organizeVersions(versionsList: Version[] | undefined, search: string | undefined) {
        if (!versionsList || !Array.isArray(versionsList)) {
            return { snapshots: [], releases: [], old: [] };
        }
        
        const searchTerm = (search || '').toLowerCase();
        const filtered = versionsList.filter(v => 
            v.id.toLowerCase().includes(searchTerm)
        );

        const snapshots = filtered.filter(v => v.version_type === 'snapshot').sort((a, b) => compareVersions(a.id, b.id));
        const releases = filtered.filter(v => v.version_type === 'release').sort((a, b) => compareVersions(a.id, b.id));
        const old = filtered.filter(v => v.version_type === 'old_beta' || v.version_type === 'old_alpha').sort((a, b) => compareVersions(a.id, b.id));

        return { snapshots, releases, old };
    }

    async function installVersionHandler() {
        if (!selectedVersion) {
            setError('Please select a version');
            return;
        }

        const version = versions.find(v => v.id === selectedVersion && v.installed);
        if (version) {
            // Version is already installed - ask if user wants to reinstall
            if (!confirm(`Version ${selectedVersion} is already installed. Replace and download again?`)) {
                return;
            }
            
            // Delete the version first
            try {
                addDownloadLog(`[INFO] Removing existing ${selectedVersion} to reinstall...`);
                await invoke('delete_version', { version: selectedVersion });
                addDownloadLog(`[OK] Removed ${selectedVersion}`);
                await loadVersions();
            } catch (err) {
                setError(`Failed to remove version: ${err}`);
                addDownloadLog(`[ERROR] Failed to remove: ${err}`);
                return;
            }
        }

        setInstallingVersion(selectedVersion);
        setInstallProgress(0);
        setInstallStatus('Starting installation...');
        setError('');
        dropdownOpen = false;
        setDownloadLogs([`Installing ${selectedVersion}...`, '']);

        try {
            addDownloadLog('[1/5] Fetching version metadata...');
            setInstallStatus('Fetching metadata...');
            setInstallProgress(10);

            addDownloadLog('[2/5] Downloading client JAR...');
            setInstallStatus('Downloading files...');
            setInstallProgress(30);

            await invoke('install_version', { version: selectedVersion });

            addDownloadLog('[3/5] Downloaded libraries');
            addDownloadLog('[4/5] Downloaded assets');
            addDownloadLog('[5/5] Installing Java runtime...');
            setInstallProgress(85);

            setInstallStatus('Installation complete!');
            setInstallProgress(100);
            addDownloadLog('');
            addDownloadLog(`[OK] ${selectedVersion} installed successfully!`);
            
            await loadVersions();
            
            setTimeout(() => {
                setInstallingVersion(null);
                selectedVersion = '';
                setInstallProgress(0);
                setInstallStatus('');
            }, 2000);
        } catch (err) {
            setError(`Installation failed: ${err}`);
            addDownloadLog('');
            addDownloadLog(`[ERROR] Installation failed: ${err}`);
            setInstallingVersion(null);
            setInstallProgress(0);
            setInstallStatus('');
        }
    }

    async function deleteInstalledVersion(version: string) {
        if (!confirm(`Delete version ${version}?`)) return;

        try {
            setDownloadLogs([`Deleting ${version}...`]);
            await invoke('delete_version', { version });
            addDownloadLog(`[OK] ${version} deleted successfully`);
            await loadVersions();
        } catch (err) {
            addDownloadLog(`[ERROR] Failed to delete: ${err}`);
            setError(`Failed to delete version: ${err}`);
        }
    }

    function formatDate(dateStr: string) {
        if (!dateStr) return '';
        return new Date(dateStr).toLocaleDateString();
    }

    function selectVersion(versionId: string) {
        selectedVersion = versionId;
        dropdownOpen = false;
    }

    function switchToVersions() {
        console.log('Switching to versions tab', currentTab, '→ versions');
        currentTab = 'versions';
        console.log('After assignment:', currentTab);
    }

    function switchToProfiles() {
        console.log('Switching to profiles tab', currentTab, '→ profiles');
        currentTab = 'profiles';
        console.log('After assignment:', currentTab);
    }

    async function launchProfileHandler(profileName: string) {
        try {
            addDownloadLog(`[INFO] Launching profile '${profileName}'...`);
            await invoke('launchprocess', { profileName });
            
            // Don't show success yet - wait for backend to report actual game exit status
            addDownloadLog(`[INFO] Game process spawned, waiting for exit status...`);
        } catch (err) {
            setError(`Failed to launch profile: ${err}`);
            addDownloadLog(`[ERROR] Failed to launch: ${err}`);
        }
    }

    onMount();
</script>

<main class="h-screen w-full flex flex-col gap-4 p-4 font-roboto bg-neutral-900">
    <!-- Tab Navigation -->
    <div class="flex gap-2 px-6 pt-2">
        <button
            onclick={switchToVersions}
            class="px-4 py-2 rounded-t-lg font-semibold text-sm transition-all {currentTab == 'versions' ? 'bg-green-400 text-neutral-900' : 'bg-neutral-700 text-gray-300 hover:bg-neutral-600'}">
            Versions
        </button>
        <button
            onclick={switchToProfiles}
            class="px-4 py-2 rounded-t-lg font-semibold text-sm transition-all {currentTab == 'profiles' ? 'bg-green-400 text-neutral-900' : 'bg-neutral-700 text-gray-300 hover:bg-neutral-600'}">
            Profiles
        </button>
    </div>

    <div class="flex gap-4 flex-1 min-h-0">
        <div class="flex-1 flex flex-col gap-6 p-6 bg-neutral-800 rounded-xl overflow-y-auto">
            
            {#if currentTab == 'versions'}
            
            {#if error}
            <div class="bg-red-900/30 border border-red-500 text-red-300 px-4 py-3 rounded-lg text-sm">
                {error}
                <button onclick={() => setError('')} class="ml-2 underline">Dismiss</button>
            </div>
            {/if}

            {#if installingVersion}
            <div class="bg-neutral-700 p-4 rounded-lg">
                <div class="flex justify-between items-start mb-2">
                    <div class="text-green-400 font-bold">Installing {installingVersion}...</div>
                    <button
                        onclick={async () => {
                            await invoke('cancel_download');
                            addDownloadLog('');
                            addDownloadLog('[INFO] Download cancelled by user');
                        }}
                        class="bg-red-600 hover:bg-red-700 text-white text-xs px-3 py-1 rounded transition-colors">
                        Cancel
                    </button>
                </div>
                <div class="w-full bg-neutral-600 rounded-full h-2 mb-2">
                    <div class="bg-green-400 h-2 rounded-full transition-all" style="width: {installProgress}%"></div>
                </div>
                <div class="text-gray-300 text-sm">{installStatus}</div>
            </div>
            {:else if loading}
            <div class="text-center text-gray-300">
                <div class="inline-block animate-spin">⟳</div> Loading versions...
            </div>
            {:else}
            <div class="flex flex-col gap-3">
                <div class="flex flex-col gap-1">
                    <label for="search-versions" class="text-green-400 text-xs uppercase tracking-widest font-bold">Search Versions</label>
                    <input 
                        id="search-versions"
                        type="text"
                        bind:value={searchQuery}
                        placeholder="Search by version ID..."
                        class="bg-neutral-900 text-white text-sm py-2 px-3 rounded-lg focus:outline-none focus:ring-2 focus:ring-green-400" 
                    />
                </div>

                <div class="flex flex-col gap-1">
                    <div class="text-green-400 text-xs uppercase tracking-widest font-bold">Select Version</div>
                    <button
                        id="select-version"
                        aria-label="Select Version"
                        onclick={() => dropdownOpen = !dropdownOpen}
                        class="bg-neutral-900 text-white text-sm font-medium py-2 px-3 rounded-lg w-full flex items-center justify-between hover:bg-neutral-700 transition-all focus:ring-0 focus:outline-none">
                        {selectedVersion || 'Select Version'}
                        <i class="fi fi-rr-angle-down text-xs transition-transform {dropdownOpen ? 'rotate-180' : ''}"></i>
                    </button>

                    {#if dropdownOpen}
                    <div class="absolute mt-10 w-80 bg-neutral-900 rounded-lg shadow-lg z-50" style="max-height: 400px; overflow-y: auto; scrollbar-width: thin; scrollbar-color: #4ade80 #171717;">
                        {#if organizedVersions.releases.length > 0}
                        <div>
                            <div class="sticky top-0 bg-neutral-800 px-3 py-2 text-green-400 text-xs font-bold uppercase">Releases ({organizedVersions.releases.length})</div>
                            {#each organizedVersions.releases as version (version.id)}
                            <button
                                onmousedown={() => selectVersion(version.id)}
                                class="w-full text-left px-3 py-2 text-sm text-gray-400 hover:text-green-400 hover:bg-neutral-700 transition-colors {selectedVersion === version.id ? 'text-green-400 bg-neutral-700' : ''} {version.installed ? 'opacity-50' : ''}">
                                {version.id} {version.installed ? '✓' : ''}
                            </button>
                            {/each}
                        </div>
                        {/if}

                        {#if organizedVersions.snapshots.length > 0}
                        <div>
                            <div class="sticky top-0 bg-neutral-800 px-3 py-2 text-yellow-400 text-xs font-bold uppercase">Snapshots ({organizedVersions.snapshots.length})</div>
                            {#each organizedVersions.snapshots as version (version.id)}
                            <button
                                onmousedown={() => selectVersion(version.id)}
                                class="w-full text-left px-3 py-2 text-sm text-gray-400 hover:text-yellow-400 hover:bg-neutral-700 transition-colors {selectedVersion === version.id ? 'text-yellow-400 bg-neutral-700' : ''} {version.installed ? 'opacity-50' : ''}">
                                {version.id} {version.installed ? '✓' : ''}
                            </button>
                            {/each}
                        </div>
                        {/if}

                        {#if organizedVersions.old.length > 0}
                        <div>
                            <div class="sticky top-0 bg-neutral-800 px-3 py-2 text-gray-400 text-xs font-bold uppercase">Old Versions ({organizedVersions.old.length})</div>
                            {#each organizedVersions.old as version (version.id)}
                            <button
                                onmousedown={() => selectVersion(version.id)}
                                class="w-full text-left px-3 py-2 text-sm text-gray-500 hover:text-gray-300 hover:bg-neutral-700 transition-colors {selectedVersion === version.id ? 'text-gray-300 bg-neutral-700' : ''} {version.installed ? 'opacity-50' : ''}">
                                {version.id} {version.installed ? '✓' : ''}
                            </button>
                            {/each}
                        </div>
                        {/if}

                        {#if organizedVersions.releases.length === 0 && organizedVersions.snapshots.length === 0 && organizedVersions.old.length === 0}
                        <div class="px-3 py-4 text-gray-400 text-sm text-center">
                            No versions found
                        </div>
                        {/if}
                    </div>
                    {/if}
                </div>

                <button
                    onclick={installVersionHandler}
                    disabled={!selectedVersion || !!installingVersion}
                    class="bg-green-400 text-neutral-900 font-bold text-sm py-2 rounded-lg w-full flex items-center justify-center gap-2 shadow-lg shadow-green-400/30 hover:bg-green-500 transition-all disabled:opacity-50 disabled:cursor-not-allowed">
                    {#if selectedVersion && versions.find(v => v.id === selectedVersion && v.installed)}
                        <i class="fi fi-rr-refresh"></i> Reinstall
                    {:else}
                        <i class="fi fi-rr-download"></i> Download & Install
                    {/if}
                </button>
            </div>
            {/if}

            <div class="border-t border-neutral-700 pt-4">
                <div class="text-green-400 text-xs uppercase tracking-widest font-bold mb-3">Installed Versions ({versions.filter(v => v.installed).length})</div>
                <div class="flex flex-col gap-2 max-h-48 overflow-y-auto">
                    {#each versions.filter(v => v.installed) as version}
                    <div class="bg-neutral-900 rounded-lg p-3 flex justify-between items-center">
                        <div>
                            <div class="text-white font-semibold">{version.id}</div>
                            <div class="text-gray-400 text-xs">{formatDate(version.release_time)}</div>
                        </div>
                        <button
                            onclick={() => deleteInstalledVersion(version.id)}
                            class="bg-red-900/30 text-red-400 hover:bg-red-900/50 px-3 py-1 rounded text-xs transition-colors">
                            Delete
                        </button>
                    </div>
                    {/each}
                    {#if versions.filter(v => v.installed).length === 0}
                    <div class="text-gray-400 text-sm text-center py-4">No versions installed</div>
                    {/if}
                </div>
            </div>
            
            {:else if currentTab == 'profiles'}
            
            <!-- Profile Creation Modal -->
            {#if showProfileModal}
            <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
                <div class="bg-neutral-800 rounded-xl p-8 w-full max-w-2xl max-h-[85vh] overflow-y-auto shadow-2xl">
                    <div class="flex justify-between items-center mb-8">
                        <h2 class="text-2xl font-bold text-green-400">Create New Profile</h2>
                        <button onclick={() => showProfileModal = false} class="text-gray-400 hover:text-white text-3xl font-light">×</button>
                    </div>
                    
                    <div class="flex flex-col gap-6">
                        <!-- Profile Name -->
                        <div>
                            <label for="profile-name" class="text-white text-sm font-semibold mb-3 block">Profile Name</label>
                            <input
                                id="profile-name"
                                type="text"
                                bind:value={profileName}
                                placeholder="e.g., My Experiment"
                                class="bg-neutral-900 text-white text-sm py-3 px-4 rounded-lg w-full focus:outline-none focus:ring-2 focus:ring-green-400"/>
                        </div>

                        <!-- Version Selection (same as Versions tab) -->
                        <div>
                            <div class="text-white text-sm font-semibold mb-3 block">Select Minecraft Version</div>
                            
                            <!-- Search Box -->
                            <input 
                                type="text"
                                bind:value={profileSearchQuery}
                                placeholder="Search version..."
                                class="bg-neutral-900 text-white text-sm py-2 px-3 rounded-lg mb-3 w-full focus:outline-none focus:ring-2 focus:ring-green-400" 
                            />

                            <!-- Version Dropdown Button -->
                            <button
                                aria-label="Select Minecraft Version"
                                onclick={() => profileDropdownOpen = !profileDropdownOpen}
                                class="bg-neutral-900 text-white text-sm font-medium py-3 px-4 rounded-lg w-full flex items-center justify-between hover:bg-neutral-700 transition-all focus:ring-0 focus:outline-none">
                                {profileSelectedVersion || 'Select Version'}
                                <i class="fi fi-rr-angle-down text-xs transition-transform {profileDropdownOpen ? 'rotate-180' : ''}"></i>
                            </button>

                            <!-- Dropdown Menu -->
                            {#if profileDropdownOpen}
                            <div class="absolute mt-2 w-80 bg-neutral-900 rounded-lg shadow-lg z-50" style="max-height: 400px; overflow-y: auto; scrollbar-width: thin; scrollbar-color: #4ade80 #171717;">
                                {#if organizedVersions.releases.filter(v => v.id.toLowerCase().includes(profileSearchQuery.toLowerCase())).length > 0}
                                <div>
                                    <div class="sticky top-0 bg-neutral-800 px-4 py-2 text-green-400 text-xs font-bold uppercase">Releases</div>
                                    {#each organizedVersions.releases.filter(v => v.id.toLowerCase().includes(profileSearchQuery.toLowerCase())) as version (version.id)}
                                    <button
                                        onmousedown={() => {
                                            profileSelectedVersion = version.id;
                                            profileDropdownOpen = false;
                                            loadModLoaderVersions(version.id);
                                        }}
                                        class="w-full text-left px-4 py-2 text-sm text-gray-400 hover:text-green-400 hover:bg-neutral-700 transition-colors {profileSelectedVersion === version.id ? 'text-green-400 bg-neutral-700' : ''}">
                                        {version.id} {version.installed ? '[Downloaded]' : ''}
                                    </button>
                                    {/each}
                                </div>
                                {/if}

                                {#if organizedVersions.snapshots.filter(v => v.id.toLowerCase().includes(profileSearchQuery.toLowerCase())).length > 0}
                                <div>
                                    <div class="sticky top-0 bg-neutral-800 px-4 py-2 text-yellow-400 text-xs font-bold uppercase">Snapshots</div>
                                    {#each organizedVersions.snapshots.filter(v => v.id.toLowerCase().includes(profileSearchQuery.toLowerCase())) as version (version.id)}
                                    <button
                                        onmousedown={() => {
                                            profileSelectedVersion = version.id;
                                            profileDropdownOpen = false;
                                            loadModLoaderVersions(version.id);
                                        }}
                                        class="w-full text-left px-4 py-2 text-sm text-gray-400 hover:text-yellow-400 hover:bg-neutral-700 transition-colors {profileSelectedVersion === version.id ? 'text-yellow-400 bg-neutral-700' : ''}">
                                        {version.id} {version.installed ? '[Downloaded]' : ''}
                                    </button>
                                    {/each}
                                </div>
                                {/if}

                                {#if organizedVersions.old.filter(v => v.id.toLowerCase().includes(profileSearchQuery.toLowerCase())).length > 0}
                                <div>
                                    <div class="sticky top-0 bg-neutral-800 px-4 py-2 text-gray-400 text-xs font-bold uppercase">Old Versions</div>
                                    {#each organizedVersions.old.filter(v => v.id.toLowerCase().includes(profileSearchQuery.toLowerCase())) as version (version.id)}
                                    <button
                                        onmousedown={() => {
                                            profileSelectedVersion = version.id;
                                            profileDropdownOpen = false;
                                            loadModLoaderVersions(version.id);
                                        }}
                                        class="w-full text-left px-4 py-2 text-sm text-gray-500 hover:text-gray-300 hover:bg-neutral-700 transition-colors {profileSelectedVersion === version.id ? 'text-gray-300 bg-neutral-700' : ''}">
                                        {version.id} {version.installed ? '[Downloaded]' : ''}
                                    </button>
                                    {/each}
                                </div>
                                {/if}

                                {#if organizedVersions.releases.length === 0 && organizedVersions.snapshots.length === 0 && organizedVersions.old.length === 0}
                                <div class="px-4 py-4 text-gray-400 text-sm text-center">
                                    No versions found
                                </div>
                                {/if}
                            </div>
                            {/if}

                            {#if profileSelectedVersion}
                            {@const selected = versions.find(v => v.id === profileSelectedVersion)}
                            {#if selected?.installed}
                            <p class="text-green-400 text-xs mt-2">✓ [Downloaded] - Already installed</p>
                            {:else}
                            <p class="text-blue-400 text-xs mt-2">⬇️ Will be downloaded automatically when profile is created</p>
                            {/if}
                            {/if}
                        </div>

                        <!-- Modloader Selection -->
                        <div>
                            <div class="text-white text-sm font-semibold mb-3 block">Modloader (Optional)</div>
                            {#if loadingModLoaders}
                            <div class="text-gray-400 text-sm py-3 px-4 bg-neutral-900 rounded-lg">
                                <div class="inline-block animate-spin">⟳</div> Loading modloaders...
                            </div>
                            {:else if profileSelectedVersion}
                            <div class="flex gap-2 mb-3">
                                <button
                                    onclick={() => { selectedModloader = 'none'; modloaderDropdownOpen = false; }}
                                    class="flex-1 py-2 px-3 rounded-lg text-sm font-medium transition-all {selectedModloader === 'none' ? 'bg-green-400 text-neutral-900' : 'bg-neutral-700 text-gray-300 hover:bg-neutral-600'}">
                                    None
                                </button>
                                <button
                                    onclick={() => { selectedModloader = 'fabric'; selectedModloaderVersion = fabricVersions[0]?.version || ''; }}
                                    disabled={!supportsFabric(profileSelectedVersion) || fabricVersions.length === 0}
                                    title={!supportsFabric(profileSelectedVersion) ? 'Fabric requires Minecraft 1.19+' : ''}
                                    class="flex-1 py-2 px-3 rounded-lg text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed {selectedModloader === 'fabric' ? 'bg-blue-500 text-white' : 'bg-neutral-700 text-gray-300 hover:bg-neutral-600'}">
                                    Fabric {fabricVersions.length > 0 ? `(${fabricVersions.length})` : ''}
                                </button>
                            </div>

                            {#if selectedModloader !== 'none'}
                            <div>
                                <div class="text-gray-300 text-xs mb-2">Select {selectedModloader} version:</div>
                                <div class="relative">
                                    <button
                                        onclick={() => modloaderDropdownOpen = !modloaderDropdownOpen}
                                        class="w-full bg-neutral-900 text-white text-sm py-2 px-3 rounded-lg flex items-center justify-between hover:bg-neutral-700 transition-all">
                                        {selectedModloaderVersion || `Select ${selectedModloader} version`}
                                        <i class="fi fi-rr-angle-down text-xs transition-transform {modloaderDropdownOpen ? 'rotate-180' : ''}"></i>
                                    </button>

                                    {#if modloaderDropdownOpen}
                                    <div class="absolute top-full left-0 right-0 mt-1 bg-neutral-900 rounded-lg shadow-lg z-50" style="max-height: 200px; overflow-y: auto; scrollbar-width: thin; scrollbar-color: #4ade80 #171717;">
                                        {#if selectedModloader === 'fabric'}
                                            {#each fabricVersions as version (version.version)}
                                            <button
                                                onmousedown={() => {
                                                    selectedModloaderVersion = version.version;
                                                    modloaderDropdownOpen = false;
                                                }}
                                                class="w-full text-left px-3 py-2 text-sm text-gray-400 hover:text-blue-400 hover:bg-neutral-700 transition-colors {selectedModloaderVersion === version.version ? 'text-blue-400 bg-neutral-700' : ''}">
                                                {version.version} {version.stable ? '(stable)' : '(beta)'}
                                            </button>
                                            {/each}
                                        {:else if selectedModloader === 'forge'}
                                            {#each forgeVersions as version (version.version)}
                                            <button
                                                onmousedown={() => {
                                                    selectedModloaderVersion = version.version;
                                                    modloaderDropdownOpen = false;
                                                }}
                                                class="w-full text-left px-3 py-2 text-sm text-gray-400 hover:text-orange-400 hover:bg-neutral-700 transition-colors {selectedModloaderVersion === version.version ? 'text-orange-400 bg-neutral-700' : ''}">
                                                {version.version} {version.latest ? '(latest)' : ''}
                                            </button>
                                            {/each}
                                        {/if}
                                    </div>
                                    {/if}
                                </div>
                            </div>
                            {/if}
                            {:else}
                            <p class="text-gray-400 text-xs py-3">Select a Minecraft version first to see available modloaders</p>
                            {/if}
                        </div>
                    </div>

                    <!-- Action Buttons -->
                    <div class="flex gap-3 pt-8 mt-8 border-t border-neutral-700">
                        <button
                            onclick={() => {
                                showProfileModal = false;
                                profileName = '';
                                profileSelectedVersion = '';
                                selectedModloader = 'none';
                                selectedModloaderVersion = '';
                                profileSearchQuery = '';
                                profileDropdownOpen = false;
                                modloaderDropdownOpen = false;
                            }}
                            class="flex-1 bg-neutral-700 hover:bg-neutral-600 text-white font-semibold py-3 rounded-lg transition-colors">
                            Cancel
                        </button>
                        <button
                            onclick={createProfileHandler}
                            disabled={!profileName.trim() || !profileSelectedVersion || creatingProfile}
                            class="flex-1 bg-green-400 hover:bg-green-500 disabled:bg-gray-600 text-neutral-900 font-bold py-3 rounded-lg transition-colors disabled:cursor-not-allowed flex items-center justify-center gap-2">
                            {#if creatingProfile}
                            <div class="animate-spin">⟳</div>
                            <span>Creating...</span>
                            {:else}
                            <i class="fi fi-rr-check"></i>
                            <span>Create Profile</span>
                            {/if}
                        </button>
                    </div>
                </div>
            </div>
            {/if}
            
            <!-- Profile Settings Modal -->
            {#if showProfileSettings}
            <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
                <div class="bg-neutral-800 rounded-xl p-8 w-full max-w-lg shadow-2xl">
                    <div class="flex justify-between items-center mb-8">
                        <h2 class="text-2xl font-bold text-blue-400">Profile Settings</h2>
                        <button onclick={() => showProfileSettings = false} class="text-gray-400 hover:text-white text-3xl font-light">×</button>
                    </div>
                    
                    <div class="flex flex-col gap-6">
                        <!-- Profile Name Display -->
                        <div>
                            <div class="text-white text-sm font-semibold mb-3 block">Profile: {settingsProfileName}</div>
                        </div>

                        <!-- RAM Setting -->
                        <div>
                            <div class="flex justify-between items-center mb-3">
                                <label for="ram-slider" class="text-white text-sm font-semibold">Memory (RAM)</label>
                                <span class="text-green-400 font-bold text-lg">{settingsRamMb}MB</span>
                            </div>
                            <div class="flex items-center gap-4">
                                <span class="text-gray-400 text-xs">512MB</span>
                                <input 
                                    id="ram-slider"
                                    type="range" 
                                    bind:value={settingsRamMb}
                                    min={512}
                                    max={16384}
                                    step={256}
                                    class="flex-1 h-2 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-green-400"
                                />
                                <span class="text-gray-400 text-xs">16GB</span>
                            </div>
                            <p class="text-gray-500 text-xs mt-2">Recommended: 4096-8192MB for modded instances</p>
                        </div>
                    </div>

                    <!-- Action Buttons -->
                    <div class="flex gap-3 pt-8 mt-8 border-t border-neutral-700">
                        <button
                            onclick={() => showProfileSettings = false}
                            class="flex-1 bg-neutral-700 hover:bg-neutral-600 text-white font-semibold py-3 rounded-lg transition-colors">
                            Cancel
                        </button>
                        <button
                            onclick={saveRamSettings}
                            class="flex-1 bg-blue-400 hover:bg-blue-500 text-neutral-900 font-bold py-3 rounded-lg transition-colors flex items-center justify-center gap-2">
                            Save Settings
                        </button>
                    </div>
                </div>
            </div>
            {/if}
            
            <!-- Profiles List -->
            <div class="flex flex-col gap-4">
                <button
                    onclick={() => showProfileModal = true}
                    class="bg-green-400 text-neutral-900 font-bold text-sm py-2 rounded-lg flex items-center justify-center gap-2 shadow-lg shadow-green-400/30 hover:bg-green-500 transition-all">
                    <i class="fi fi-rr-plus"></i> Create Profile
                </button>
                
                {#if profiles.length === 0}
                <div class="text-center py-8 text-gray-400">
                    <div class="text-lg mb-2">No profiles yet</div>
                    <div class="text-sm">Create a profile to get started</div>
                </div>
                {:else}
                <div class="flex flex-col gap-3">
                    {#each profiles as profile (profile.name)}
                    <div class="bg-neutral-900 rounded-lg p-4 border border-neutral-700 hover:border-green-400/50 transition-colors">
                        <div class="flex justify-between items-start mb-2">
                            <div>
                                <div class="text-white font-semibold text-lg">{profile.name}</div>
                                <div class="text-gray-400 text-sm">Version: {profile.base_version}</div>
                            </div>
                            <div class="flex gap-2">
                                <button
                                    onclick={() => launchProfileHandler(profile.name)}
                                    class="bg-green-500/20 text-green-400 hover:bg-green-500/40 active:bg-green-500/60 px-3 py-1 rounded text-sm transition-colors font-semibold">
                                    Launch
                                </button>
                                <button
                                    onclick={() => openProfileSettings(profile)}
                                    class="bg-blue-900/30 text-blue-400 hover:bg-blue-900/50 px-3 py-1 rounded text-sm transition-colors">
                                    Settings
                                </button>
                                <button
                                    onclick={() => deleteProfileHandler(profile.name)}
                                    class="bg-red-900/30 text-red-400 hover:bg-red-900/50 px-3 py-1 rounded text-sm transition-colors">
                                    Delete
                                </button>
                            </div>
                        </div>
                        <div class="text-gray-500 text-xs flex gap-4">
                            {#if profile.ram_mb}
                            <span>RAM: {profile.ram_mb}MB</span>
                            {/if}
                            {#if profile.last_played}
                            <span>Last played: {formatDate(profile.last_played)}</span>
                            {:else}
                            <span>Never played</span>
                            {/if}
                        </div>
                    </div>
                    {/each}
                </div>
                {/if}
            </div>
            
            {/if}
        </div>

        <div class="w-80 flex flex-col p-6 bg-neutral-800 rounded-xl h-full">
            <div class="text-green-400 text-xs uppercase tracking-widest font-bold mb-3 shrink-0">Activity Log</div>
            <div bind:this={logContainer} class="flex-1 bg-neutral-900 rounded-lg p-4 overflow-y-auto text-xs font-mono text-gray-300 space-y-1 min-h-0">
                {#each downloadLogs as log}
                <div class="whitespace-pre-wrap break-all">{log}</div>
                {/each}
                {#if downloadLogs.length === 0}
                <div class="text-gray-500">Waiting for activity...</div>
                {/if}
            </div>
        </div>
    </div>
</main>

<style>
    :global(body) {
        margin: 0;
        padding: 0;
    }
</style>