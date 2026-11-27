<script lang="ts">
	import type { AcquisitionKind } from '#lib/api/types.ts';

	// How something was acquired: shared by spools and equipment forms. Sits
	// inside a two-column grid.
	let {
		acquisition = $bindable(),
		date = $bindable(),
		from = $bindable(),
		price = $bindable(),
		discounted = $bindable(),
		discountNote = $bindable(),
		input,
		label
	}: {
		acquisition: AcquisitionKind;
		date: string;
		from: string;
		price: number | null;
		discounted: boolean;
		discountNote: string;
		input: string;
		label: string;
	} = $props();
</script>

<label>
	<span class={label}>Acquired</span>
	<select bind:value={acquisition} class={input}>
		<option value="bought">Bought</option>
		<option value="gift">Gift</option>
		<option value="other">Other</option>
	</select>
</label>
<label>
	<span class={label}>On</span>
	<input type="date" bind:value={date} class={input} />
</label>
<label>
	<span class={label}>From</span>
	<input
		type="text"
		bind:value={from}
		placeholder={acquisition === 'gift' ? 'Who gave it' : 'Store, website or person'}
		class={input}
	/>
</label>
<label>
	<span class={label}>Price, if known</span>
	<input type="number" inputmode="decimal" min="0" step="0.01" bind:value={price} class={input} />
</label>
<label class="col-span-2 flex items-center gap-2 text-sm">
	<input type="checkbox" bind:checked={discounted} class="size-4 accent-orange-600" />
	Bought at a discount
</label>
{#if discounted}
	<label class="col-span-2">
		<span class={label}>Discount</span>
		<input
			type="text"
			bind:value={discountNote}
			placeholder="Black Friday, 30% off"
			class={input}
		/>
	</label>
{/if}
