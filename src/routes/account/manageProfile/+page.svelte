<script>
    import { Button } from "flowbite-svelte";
</script>

<main>
    <div class="flex flex-row p-3">
        <ol class="w-full flex flex-col gap-5">
            <li class="w-full flex flex-row text-white font-roboto text-2xl font-medium py-1 px-10 bg-neutral-800 rounded-2xl justify-between shadow-lg/60">

                <span class="p-5 text-shadow-lg">Regular_User</span>
                <div class="flex flex-1 gap-5 m-auto place-content-end align-baseline ml-auto items-center">
                    <button class="text-shadow-lg text-xl text-white px-10 py-5 font-roboto font-medium p-2 ease-in-out transition-colors transition-transform duration-500 bg-neutral-900 hover:bg-green-500 active:bg-green-900 hover:scale-105">Set</button>
                    <button class="text-shadow-lg text-xl text-white px-10 py-5 font-roboto font-medium p-2 bg-neutral-900 ease-in-out transition-colors transition-transform duration-500 hover:bg-red-500 active:bg-red-900 hover:scale-105">Delete</button>
                </div>

            </li>
            
        </ol>
    </div>
</main>