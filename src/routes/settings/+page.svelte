<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';

    interface GameSettings {
        vanilla_ram_mb: number;
        custom_jvm_args: string;
        game_width: number;
        game_height: number;
        fullscreen: boolean;
        close_launcher_on_start: boolean;
        keep_launcher_background: boolean;
    }

    let settings = $state<GameSettings>({
        vanilla_ram_mb: 2048,
        custom_jvm_args: '',
        game_width: 854,
        game_height: 480,
        fullscreen: false,
        close_launcher_on_start: false,
        keep_launcher_background: false,
    });

    let ramValue = $state(2048);
    let loading = $state(true);
    let error = $state('');
    let success = $state('');
    let showResetConfirm = $state(false);

    async function loadSettings() {
        try {
            const loaded = await invoke<GameSettings>('load_game_settings');
            settings = loaded;
            ramValue = loaded.vanilla_ram_mb;
            loading = false;
        } catch (err) {
            error = `Failed to load settings: ${err}`;
            loading = false;
        }
    }

    async function saveSettings() {
        try {
            error = '';
            settings.vanilla_ram_mb = ramValue;
            await invoke('save_game_settings', { settings });
            success = 'Settings saved successfully!';
            setTimeout(() => { success = ''; }, 3000);
        } catch (err) {
            error = `Failed to save settings: ${err}`;
        }
    }

    async function resetSettings() {
        try {
            error = '';
            const defaultSettings = await invoke<GameSettings>('reset_game_settings');
            settings = defaultSettings;
            ramValue = defaultSettings.vanilla_ram_mb;
            showResetConfirm = false;
            success = 'Settings reset to defaults!';
            setTimeout(() => { success = ''; }, 3000);
        } catch (err) {
            error = `Failed to reset settings: ${err}`;
            showResetConfirm = false;
        }
    }

    async function quitLauncher() {
        try {
            await invoke('quit_app');
        } catch (err) {
            error = `Failed to quit: ${err}`;
        }
    }

    onMount();

    function onMount() {
        loadSettings();
    }
</script>

<main class="w-full p-6">
    <div class="grid grid-cols-1 gap-6 text-xl text-white font-roboto font-medium max-w-2xl">
        <h1 class="text-green-400 text-2xl font-bold">Java & Game Settings</h1>

        {#if error}
        <div class="bg-red-900/30 border border-red-500 text-red-300 px-4 py-3 rounded-lg text-sm">
            {error}
        </div>
        {/if}

        {#if success}
        <div class="bg-green-900/30 border border-green-500 text-green-300 px-4 py-3 rounded-lg text-sm">
            {success}
        </div>
        {/if}

        {#if loading}
        <div class="text-gray-400">Loading settings...</div>
        {:else}
        <!-- Allocate Memory -->
        <div class="flex flex-col gap-2">
            <label class="text-green-400 font-bold text-sm">Allocate Memory (Default/Vanilla Minecraft)</label>
            <div class="flex items-center gap-4">
                <input 
                    type="range" 
                    bind:value={ramValue}
                    min="512" 
                    max="16384" 
                    step="256"
                    class="flex-1 accent-green-400 h-2"
                />
                <span class="text-white font-bold min-w-24">{ramValue} MB</span>
            </div>
            <p class="text-gray-400 text-xs">Note: Profile RAM settings override this value when launching profiles</p>
        </div>

        <!-- Custom JVM Arguments -->
        <div class="flex flex-col gap-2">
            <label class="text-green-400 font-bold text-sm">Custom JVM Arguments</label>
            <input 
                bind:value={settings.custom_jvm_args}
                placeholder="e.g., -XX:+UseG1GC -XX:MaxGCPauseMillis=200"
                autocomplete="off"
                class="bg-neutral-800 border-none text-white text-sm transition-all duration-400 hover:ring-1 ring-green-400 px-3 py-2 rounded"
            />
            <p class="text-gray-400 text-xs">Separate arguments with spaces</p>
        </div>

        <!-- Game Resolution -->
        <div class="flex flex-col gap-2">
            <label class="text-green-400 font-bold text-sm">Default Game Resolution</label>
            <div class="flex gap-4">
                <div class="flex-1">
                    <label class="text-white text-xs font-bold mb-1 block">Width</label>
                    <input 
                        bind:value={settings.game_width}
                        type="number"
                        min="640"
                        max="3840"
                        placeholder="854"
                        autocomplete="off"
                        class="w-full bg-neutral-800 border-none text-white text-sm transition-all duration-400 hover:ring-1 ring-green-400 px-3 py-2 rounded"
                    />
                </div>
                <div class="flex-1">
                    <label class="text-white text-xs font-bold mb-1 block">Height</label>
                    <input 
                        bind:value={settings.game_height}
                        type="number"
                        min="480"
                        max="2160"
                        placeholder="480"
                        autocomplete="off"
                        class="w-full bg-neutral-800 border-none text-white text-sm transition-all duration-400 hover:ring-1 ring-green-400 px-3 py-2 rounded"
                    />
                </div>
            </div>
        </div>

        <!-- Toggle Options -->
        <div class="flex flex-col gap-3 pt-3">
            <div class="flex items-center justify-between">
                <label class="text-white text-sm font-bold">Fullscreen on Launch</label>
                <input 
                    type="checkbox" 
                    bind:checked={settings.fullscreen}
                    class="w-4 h-4 accent-green-400 cursor-pointer"
                />
            </div>
            
            <div class="flex items-center justify-between">
                <label class="text-white text-sm font-bold">Close Launcher When Game Starts</label>
                <input 
                    type="checkbox" 
                    bind:checked={settings.close_launcher_on_start}
                    class="w-4 h-4 accent-green-400 cursor-pointer"
                />
            </div>
            
            <div class="flex items-center justify-between">
                <label class="text-white text-sm font-bold">Keep Launcher in Background</label>
                <input 
                    type="checkbox" 
                    bind:checked={settings.keep_launcher_background}
                    class="w-4 h-4 accent-green-400 cursor-pointer"
                />
            </div>
            <p class="text-gray-400 text-xs mb-3">When enabled, closing the launcher window will minimize it instead of quitting. Use the Quit Launcher button below to completely exit.</p>
        </div>

        <!-- Save and Reset Buttons -->
        <div class="flex gap-2 pt-4">
            <button 
                onclick={saveSettings}
                class="bg-green-600 hover:bg-green-700 active:bg-green-800 text-white font-bold py-2 px-4 rounded transition-all duration-200"
            >
                Save Settings
            </button>
            <button 
                onclick={() => showResetConfirm = true}
                class="bg-red-600 hover:bg-red-700 active:bg-red-800 text-white font-bold py-2 px-4 rounded transition-all duration-200"
            >
                Reset to Defaults
            </button>
            <button 
                onclick={quitLauncher}
                class="bg-red-600 hover:bg-red-700 active:bg-red-800 text-white font-bold py-2 px-4 rounded transition-all duration-200 ml-auto"
            >
                Quit Launcher
            </button>
        </div>

        {#if showResetConfirm}
        <div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50 rounded-lg">
            <div class="bg-neutral-900 border border-red-500 p-6 rounded-lg max-w-sm">
                <h2 class="text-xl text-red-400 font-bold mb-4">Reset Settings?</h2>
                <p class="text-gray-300 mb-6">Are you sure you want to reset all settings to their default values? This cannot be undone.</p>
                <div class="flex gap-3">
                    <button 
                        onclick={resetSettings}
                        class="bg-red-600 hover:bg-red-700 active:bg-red-800 text-white font-bold py-2 px-4 rounded transition-all duration-200"
                    >
                        Yes, Reset
                    </button>
                    <button 
                        onclick={() => showResetConfirm = false}
                        class="bg-green-600 hover:bg-green-700 active:bg-green-800 text-white font-bold py-2 px-4 rounded transition-all duration-200"
                    >
                        Cancel
                    </button>
                </div>
            </div>
        </div>
        {/if}
        {/if}
    </div>
</main>