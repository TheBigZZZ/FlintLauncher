<script>
  import '../app.css';
  import { goto } from '$app/navigation';

  let { children } = $props();

  function goToAccount(){
    goto('/account')
  }

  function goToHome() {
    goto('/home')
  }


</script>

<main class="flex h-screen">
  <!-- NavBar -->
  <div class="text-center bg-neutral-800 px-5 py-5 flex flex-col w-30 gap-8 shrink-0">
    <div class="font-roboto font-bold text-2xl antialiased text-green-400 text-shadow-lg/30 transition-all hover:animate-ping">F</div>

    <div class="text-gray-500 transition-all duration-300 ease-in-out hover:text-green-400 hover:drop-shadow-green-300 drop-shadow-2xl">
      <i class="fi-rr-home text-xl text-shadow-lg/30"></i>
      <p class="font-rubik text-shadow-lg/30">Home</p>
    </div>

    <div class="text-gray-500 transition-colors ease-in-out hover:text-green-400 block hover:drop-shadow-green-300 drop-shadow-2xl">
      <i class="fi fi-rr-books-medical text-xl text-shadow-lg/30"></i>
      <p class="font-rubik text-shadow-lg/30">Library</p>
    </div>

    <div class="text-gray-500 transition-colors ease-in-out hover:text-green-400 block hover:drop-shadow-green-300 drop-shadow-2xl">
      <i class="fi fi-rr-document-gavel text-xl text-shadow-lg/30"></i>
      <p class="font-rubik text-shadow-lg/30">Mods</p>
    </div>

    <button type="button" onclick={goToAccount} class="text-gray-500 transition-colors ease-in-out hover:text-green-400 block hover:drop-shadow-green-300 drop-shadow-2xl">
      <i class="fi fi-rr-user-add text-xl text-shadow-lg/30"></i>
      <p class="font-rubik text-shadow-lg/30">Accounts</p>
    </button>

    <div class="text-gray-500 mt-auto transition-colors ease-in-out hover:text-green-400 hover:drop-shadow-green-300 drop-shadow-2xl">
      <i class="fi fi-rr-settings-sliders text-xl text-shadow-lg/30"></i>
      <p class="font-rubik text-shadow-lg/30">Settings</p>
    </div>
  </div>

  <!-- Page content -->
  <div class="flex-1 overflow-auto">
    {@render children()}
  </div>
</main>