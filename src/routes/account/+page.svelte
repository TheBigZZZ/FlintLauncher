<script lang="ts">
      import { progressbar, Progressbar } from "flowbite-svelte";
      import { Badge } from "flowbite-svelte";
      import { goto } from "$app/navigation";

      let user: string = 'Regular_User'

      function goToAddAccount(){
        goto('/account/addAccount')
      }

      function goToManageProfile(){
        goto('/account/manageProfile')
      }

</script>

<main>
    <div class="grid grid-column grid-cols-1 gap-10 p-2 justify-items-end my-10">
        <div>
            <h1 class="font-rubix text-white text-xl px-10 pb-3">Authenticated Accounts</h1>
            <h5 class="text-gray-500 font-roboto font-bold text-s px-10">Manage all your identities across Mojang ecosystems. Switching Accounts instantly updates your active profile settings.</h5>
        </div>

        <div>
            <button onclick={goToManageProfile} class="text-white text-xl p-3 font-roboto bg-neutral-800 rounded-xl transition-all hover:bg-neutral-950 active:bg-neutral-900">Manage Profiles</button>
            <button onclick={goToAddAccount} class="text-natural-900 text-xl p-3 mx-10 font-roboto bg-green-400 rounded-xl shadow-green-400/50 shadow-2xl backdrop-blur-2xl transition-all hover:bg-green-200 active:bg-neutral-400">Add Account</button>
        </div>
    </div>

        <div class="grid grid-flow-col grid-cols-auto gap-5 p-5 align-center">

            <div class="bg-neutral-800 px-6 rounded-xl">
                <h1 class="text-xl text-roboto text-green-400 py-2 font-medium">Offline Account</h1>
                <Badge rounded border large color="green">Active</Badge>
                <h3 class="text-3xl text-rubix text-white py-3 font-medium pb-6">{user}</h3>
                
                <div class="flex flex-col gap-3">
                    <button class="bg-neutral-900 text-white text-xl text-roboto font-medium p-2 rounded-lg flex items-center gap-3 transition-all hover:bg-neutral-950 active:bg-neutral-900"><i class="fi fi-rr-fill"></i>Skin Editor</button>

            <button class="bg-neutral-900 text-white text-xl text-roboto font-medium p-2 px-2  rounded-lg flex items-center gap-3 transition-all hover:bg-neutral-950 active:bg-neutral-900"><i class="fi fi-rr-sign-out-alt"></i>Sign Out</button>
                </div>
            </div>

            <div class="p-3 bg-neutral-800 px-5 py-3 rounded-xl">
                <h1 class="text-xl text-roboto text-gray-300 font-medium">Storage Allocation</h1>

                <h3 class="text-lg text-green-400 text-roboto font-medium py-2 pt-3">JVM Memory</h3>
                <Progressbar precision={2} tweenDuration={2000} animate classes={{ label: "bg-green-400" }} progress="67"/>
                <p class="text-lg text-gray-300 text-roboto font-medium py-2">0 GB / 8 GB</p>

                <h3 class="text-lg text-yellow-400 text-roboto font-medium py-2 pt-5 ">Asset Cache</h3>
                <Progressbar precision={2} tweenDuration={2000} animate classes={{ label: "bg-yellow-400" }} progress="67"/>
                <p class="text-lg text-gray-300 text-roboto font-medium py-2">0 GB</p>
            </div>

        </div>
</main>