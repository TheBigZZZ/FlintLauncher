<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { Label, Input, Button, Toggle } from "flowbite-svelte";

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
            <Label class="text-green-400 font-bold text-sm">Allocate Memory (Default/Vanilla Minecraft)</Label>
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
            <Label class="text-green-400 font-bold text-sm">Custom JVM Arguments</Label>
            <Input 
                bind:value={settings.custom_jvm_args}
                placeholder="e.g., -XX:+UseG1GC -XX:MaxGCPauseMillis=200"
                autocomplete="off"
                class="bg-neutral-800 border-none text-white text-sm transition-all duration-400 hover:ring-1 ring-green-400"
            />
            <p class="text-gray-400 text-xs">Separate arguments with spaces</p>
        </div>

        <!-- Game Resolution -->
        <div class="flex flex-col gap-2">
            <Label class="text-green-400 font-bold text-sm">Default Game Resolution</Label>
            <div class="flex gap-4">
                <div class="flex-1">
                    <Label class="text-white text-xs font-bold mb-1">Width</Label>
                    <Input 
                        bind:value={settings.game_width}
                        type="number"
                        min="640"
                        max="3840"
                        placeholder="854"
                        autocomplete="off"
                        class="bg-neutral-800 border-none text-white text-sm transition-all duration-400 hover:ring-1 ring-green-400"
                    />
                </div>
                <div class="flex-1">
                    <Label class="text-white text-xs font-bold mb-1">Height</Label>
                    <Input 
                        bind:value={settings.game_height}
                        type="number"
                        min="480"
                        max="2160"
                        placeholder="480"
                        autocomplete="off"
                        class="bg-neutral-800 border-none text-white text-sm transition-all duration-400 hover:ring-1 ring-green-400"
                    />
                </div>
            </div>
        </div>

        <!-- Toggle Options -->
        <div class="flex flex-col gap-3 pt-3">
            <div class="flex items-center justify-between">
                <Label class="text-white text-sm font-bold font-rubix">Fullscreen on Launch</Label>
                <Toggle 
                    bind:checked={settings.fullscreen}
                    color="green" 
                    class="text-white text-sm font-bold font-rubix focus:ring-0 duration-400 transition-all hover:text-green-400"
                />
            </div>
            
            <div class="flex items-center justify-between">
                <Label class="text-white text-sm font-bold font-rubix">Close Launcher When Game Starts</Label>
                <Toggle 
                    bind:checked={settings.close_launcher_on_start}
                    color="green" 
                    class="text-white text-sm font-bold font-rubix focus:ring-0 transition-all duration-400 hover:text-green-400"
                />
            </div>
            
            <div class="flex items-center justify-between">
                <Label class="text-white text-sm font-bold font-rubix">Keep Launcher in Background</Label>
                <Toggle 
                    bind:checked={settings.keep_launcher_background}
                    color="green" 
                    class="text-white text-sm font-bold font-rubix focus:ring-0 transition-all duration-400 hover:text-green-400"
                />
            </div>
            <p class="text-gray-400 text-xs mb-3">When enabled, closing the launcher window will minimize it instead of quitting. Use the Quit Launcher button below to completely exit.</p>
        </div>

        <!-- Save and Reset Buttons -->
        <div class="flex gap-2 pt-4">
            <Button 
                onclick={saveSettings}
                color="green" 
                size="md" 
                class="transition-all duration-400 active:bg-green-600 focus:ring-0 font-bold"
            >
                Save Settings
            </Button>
            <Button 
                onclick={() => showResetConfirm = true}
                color="red" 
                size="md" 
                class="transition-all duration-400 active:bg-red-600 focus:ring-0 font-bold"
            >
                Reset to Defaults
            </Button>
            <Button 
                onclick={quitLauncher}
                color="red" 
                size="md" 
                class="transition-all duration-400 active:bg-red-600 focus:ring-0 font-bold ml-auto"
            >
                Quit Launcher
            </Button>
        </div>

        {#if showResetConfirm}
        <div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50 rounded-lg">
            <div class="bg-neutral-900 border border-red-500 p-6 rounded-lg max-w-sm">
                <h2 class="text-xl text-red-400 font-bold mb-4">Reset Settings?</h2>
                <p class="text-gray-300 mb-6">Are you sure you want to reset all settings to their default values? This cannot be undone.</p>
                <div class="flex gap-3">
                    <Button 
                        onclick={resetSettings}
                        color="red" 
                        size="sm"
                        class="transition-all duration-400 active:bg-red-600 focus:ring-0 font-bold"
                    >
                        Yes, Reset
                    </Button>
                    <Button 
                        onclick={() => showResetConfirm = false}
                        color="green" 
                        size="sm"
                        class="transition-all duration-400 focus:ring-0 font-bold"
                    >
                        Cancel
                    </Button>
                </div>
            </div>
        </div>
        {/if}
        {/if}
    </div>
</main>