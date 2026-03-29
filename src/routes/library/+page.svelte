<script>
    let open = $state(false);
    const versions = ['1.21.1', '1.21.0', '1.20.4', '1.20.1', '1.19.4', '1.18.2', '1.17.1', '1.16.5', '1.15.2', '1.14.4'];
    let selected = $state('Select Version');
</script>

<main class="h-screen w-full flex items-center justify-center">
    <div class="flex flex-col gap-3 p-4 font-roboto font-medium bg-neutral-800 rounded-xl w-72">

        <div class="flex flex-col gap-1">
            <label class="text-green-400 text-xs uppercase tracking-widest">Game Engine</label>
            <select class="bg-neutral-900 text-white text-sm px-3 py-2 rounded-lg outline-none border-0 cursor-pointer appearance-none">
                <option value="Vanilla">Vanilla</option>
                <option value="Fabric">Fabric</option>
                <option value="Forge">Forge</option>
            </select>
        </div>

        <div class="flex flex-col gap-1">
            <label class="text-green-400 text-xs uppercase tracking-widest">Minecraft Version</label>
            <select class="bg-neutral-900 text-white text-sm px-3 py-2 rounded-lg outline-none border-0 cursor-pointer appearance-none">
                <option value="TEST_VERSION">TEST_VERSION</option>
            </select>
        </div>

        <button class="bg-green-400 text-neutral-900 font-bold text-sm py-2 rounded-lg w-full flex items-center justify-center gap-2 shadow-lg shadow-green-400/30 hover:bg-green-500 transition-all focus:ring-0 focus:outline-none active:bg-green-400">
            <i class="fi fi-rr-download"></i> Download & Install
        </button>

        <div class="relative">
            <button
                onclick={() => open = !open}
                class="bg-neutral-900 text-white text-sm font-medium py-2 px-3 rounded-lg w-full flex items-center justify-between hover:bg-neutral-700 transition-all focus:ring-0 focus:outline-none">
                {selected}
                <i class="fi fi-rr-angle-down text-xs transition-transform duration-200 {open ? 'rotate-180' : ''}"></i>
            </button>

            {#if open}
            <div
                class="absolute bottom-full mb-1 w-full bg-neutral-900 rounded-lg shadow-lg z-50"
                style="max-height: 160px; overflow-y: scroll; scrollbar-width: thin; scrollbar-color: #4ade80 #171717;">
                {#each versions as version}
                <button
                    onmousedown={() => { selected = version; open = false; }}
                    class="w-full text-left px-3 py-2 text-sm text-gray-400 hover:text-green-400 hover:bg-neutral-700 transition-colors {selected === version ? 'text-green-400' : ''}">
                    {version}
                </button>
                {/each}
            </div>
            {/if}
        </div>

    </div>
</main>