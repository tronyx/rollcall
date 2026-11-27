<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<div class="mx-auto min-h-dvh max-w-6xl px-4 pt-4 pb-24 sm:px-6 print:max-w-none print:p-0">
	{@render children()}
</div>
