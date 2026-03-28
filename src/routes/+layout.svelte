<script>
  import '../app.css';
  import NavBar from '../components/NavBar.svelte';
  let { children } = $props();
</script>


<main>
<NavBar></NavBar>
</main>

{@render children()}