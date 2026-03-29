<script lang="ts">
    import { Fileupload } from "flowbite-svelte";
    import { Label } from "flowbite-svelte";
    import { Input } from "flowbite-svelte";
    import { Button } from "flowbite-svelte";
    import { Toggle } from "flowbite-svelte";
</script>

<main class="w-full">
    <div class="grid grid-cols-1 p-5 gap-3 text-xl text-white font-roboto font-medium">--------- Java & Game Settings ---------
        <div>
            <Label for="with_helper" class="pb-1 text-green-400 font-bold text-sm">Java Path Picker</Label>
            <Fileupload id="with_helper" class="mb-2 bg-neutral-800 text-white border-none hover:outline-1 outline-green-400" />
        </div>
        
        <div>
            <Label class="pb-1 text-green-400 font-bold text-sm">Allocate Memory</Label>
            <input type="range" class="accent-green-400">
        </div>

        <div>
            <Label class="pb-1 text-green-400 font-bold text-sm">Custom JVM Arguments</Label>
            <Input clearable class="bg-neutral-800 border-none text-white text-sm font-rubix font-medium hover:outline-1 outline-green-400"/>
        </div>
        
        <Label class="pb-1 text-green-400 font-bold text-sm">Default Game Resolution</Label>
        
        <div class="flex flex-wrap gap-4">
            <Label class="text-white text-sm font-roboto font-medium">Width:<Input clearable class="w-full bg-neutral-800 border-none text-white text-sm font-rubix font-medium hover:outline-1 outline-green-400"/></Label>

            <Label class="text-white text-sm font-roboto font-medium">Height:<Input clearable class="w-full bg-neutral-800 border-none text-white text-sm font-rubix font-medium hover:outline-1 outline-green-400"/></Label>

            <button class="bg-neutral-800 text-white text-sm font-roboto font-medium px-8 py-2 rounded-lg transition-all hover:bg-green-400 active:bg-neutral-800">Set</button>

        </div>

        <div class="flex flex-col gap-2 pt-3 pb-3">
            <Toggle color={"green"} class="text-white text-sm font-medium font-rubix focus:ring-0">Fullscreen on Launch</Toggle>
            <Toggle color={"green"} class="text-white text-sm font-medium font-rubix focus:ring-0">Close launcher when game starts</Toggle>
            <Toggle color={"green"} class="text-white text-sm font-medium font-rubix focus:ring-0">Reopen launcher when game closes</Toggle>
            <Toggle color={"green"} class="text-white text-sm font-medium font-rubix focus:ring-0">Run launcher in background</Toggle>
        </div>

        <div>
            <Button color={'green'} size="sm" class="transition-all active:bg-green-600 focus:ring-0">Save</Button>
        </div>
    </div>

    
</main>