<script>
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";

    let accounts = $state("");

    const addAccount = async () => {
        try {
            const result = await invoke('accountcreate', { username: accounts });
            console.log('Account added successfully:', result);
        } catch (error) {
            console.error('Failed to add account:', error);
        }
    }

</script>

<main>
    <div class="flex flex-col p-5 gap-3 text-center place-items-center place-content-baseline justify-center justify-items-center h-screen">
        <label class="text-white text-2xl font-roboto">Username:</label>
        <input bind:value={accounts} class="bg-neutral-800 text-white font-roboto text-xl rounded-lg p-2 outline-none">
        <button onclick={addAccount} class="text-white font-medium font-roboto p-2 px-5 text-centeer bg-neutral-800 rounded-xl transition-all hover:bg-green-500 active:bg-neutral-800">Add User</button>
    </div>
</main>