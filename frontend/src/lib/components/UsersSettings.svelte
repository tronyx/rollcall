<script lang="ts">
	import { api, ApiError } from '#lib/api/client.ts';
	import type { User } from '#lib/api/types.ts';
	import { session } from '#lib/state/session.svelte.ts';
	import { dayOf } from '#lib/util/format.ts';
	import { onMount } from 'svelte';

	let users = $state<User[]>([]);
	let registration = $state(false);
	let loaded = $state(false);

	let adding = $state(false);
	let newName = $state('');
	let newPassword = $state('');

	// The account whose password is being reset, or being confirmed for removal.
	let resetId = $state<string | null>(null);
	let resetPassword = $state('');
	let removeId = $state<string | null>(null);
	let notice = $state<string | null>(null);

	let busy = $state(false);
	let error = $state<string | null>(null);

	const message = (e: unknown, fallback: string) => (e instanceof ApiError ? e.message : fallback);
	const byName = (list: User[]) =>
		[...list].sort((a, b) => a.username.toLowerCase().localeCompare(b.username.toLowerCase()));

	onMount(async () => {
		try {
			const [u, s] = await Promise.all([api.users(), api.adminSettings()]);
			users = u;
			registration = s.registration_enabled;
			loaded = true;
		} catch (e) {
			error = message(e, 'Could not load the accounts.');
		}
	});

	async function run(action: () => Promise<void>, fallback: string) {
		busy = true;
		error = null;
		notice = null;
		try {
			await action();
		} catch (e) {
			error = message(e, fallback);
		} finally {
			busy = false;
		}
	}

	const toggleRegistration = () =>
		run(async () => {
			registration = (await api.updateAdminSettings({ registration_enabled: !registration }))
				.registration_enabled;
		}, 'Could not change registration.');

	const addUser = (e: SubmitEvent) => {
		e.preventDefault();
		return run(async () => {
			const user = await api.addUser({ username: newName.trim(), password: newPassword });
			users = byName([...users, user]);
			adding = false;
			newName = '';
			newPassword = '';
		}, 'Could not add the account.');
	};

	const setAdmin = (u: User, isAdmin: boolean) =>
		run(async () => {
			const updated = await api.setAdmin(u.id, isAdmin);
			users = users.map((x) => (x.id === updated.id ? updated : x));
		}, 'Could not change the account.');

	const saveReset = (e: SubmitEvent, u: User) => {
		e.preventDefault();
		return run(async () => {
			await api.resetPassword(u.id, resetPassword);
			resetId = null;
			resetPassword = '';
			notice = `${u.username}'s password is changed, and they're signed out everywhere.`;
		}, 'Could not reset the password.');
	};

	const remove = (u: User) =>
		run(async () => {
			await api.deleteUser(u.id);
			users = users.filter((x) => x.id !== u.id);
			removeId = null;
		}, 'Could not remove the account.');

	const input =
		'mt-1 w-full rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700';
	const label = 'block text-xs font-medium text-slate-600 dark:text-slate-300';
	const button =
		'rounded-lg px-3 py-1.5 text-sm font-medium ring-1 ring-slate-300 hover:bg-slate-50 disabled:opacity-50 dark:ring-slate-700 dark:hover:bg-slate-800';
	const primary =
		'rounded-lg bg-orange-600 px-3 py-1.5 text-sm font-semibold text-white disabled:opacity-50';
</script>

<div class="flex flex-wrap items-center justify-between gap-2">
	<h2 class="font-semibold">Accounts</h2>
	{#if loaded && !adding}
		<button type="button" class={button} onclick={() => (adding = true)}>+ Add account</button>
	{/if}
</div>

{#if loaded}
	<label class="mt-3 flex items-start gap-3">
		<input
			type="checkbox"
			checked={registration}
			disabled={busy}
			onchange={toggleRegistration}
			class="mt-1 size-4 accent-orange-600"
		/>
		<span>
			<span class="block text-sm font-medium">Let people create their own accounts</span>
			<span class="block text-xs text-slate-500 dark:text-slate-400">
				When this is off, the login page has no sign-up, and only you can add accounts here.
			</span>
		</span>
	</label>

	{#if adding}
		<form
			onsubmit={addUser}
			class="mt-3 grid gap-3 rounded-xl bg-slate-50 p-3 sm:grid-cols-[1fr_1fr_auto] sm:items-end dark:bg-slate-800/50"
		>
			<label>
				<span class={label}>Username</span>
				<input type="text" bind:value={newName} required maxlength="40" class={input} />
			</label>
			<label>
				<span class={label}>Password</span>
				<input
					type="text"
					bind:value={newPassword}
					required
					minlength="8"
					autocomplete="off"
					class={input}
				/>
			</label>
			<span class="flex gap-2">
				<button type="submit" disabled={busy} class={primary}>Add</button>
				<button type="button" class={button} onclick={() => (adding = false)}>Cancel</button>
			</span>
		</form>
	{/if}

	<ul class="mt-3 divide-y divide-slate-200 text-sm dark:divide-slate-800">
		{#each users as u (u.id)}
			{@const me = u.id === session.user?.id}
			<li class="py-2">
				<div class="flex flex-wrap items-center justify-between gap-2">
					<div>
						<span class="font-medium">{u.username}</span>
						{#if u.is_admin}<span
								class="ml-1 rounded-full bg-orange-100 px-2 py-0.5 text-xs font-medium text-orange-800 dark:bg-orange-950 dark:text-orange-200"
							>
								Admin
							</span>{/if}
						{#if me}<span class="text-xs text-slate-500"> (you)</span>{/if}
						<div class="text-xs text-slate-500 dark:text-slate-400">
							Joined {dayOf(u.created_at)}
						</div>
					</div>
					{#if removeId === u.id}
						<span class="flex flex-wrap items-center gap-2">
							<span class="text-xs">Remove {u.username} and their whole inventory?</span>
							<button
								type="button"
								disabled={busy}
								class="rounded-lg bg-red-600 px-3 py-1.5 text-sm font-medium text-white"
								onclick={() => remove(u)}
							>
								Remove
							</button>
							<button type="button" class={button} onclick={() => (removeId = null)}>Keep</button>
						</span>
					{:else}
						<span class="flex flex-wrap gap-2">
							{#if !me}
								<button
									type="button"
									disabled={busy}
									class={button}
									onclick={() => setAdmin(u, !u.is_admin)}
								>
									{u.is_admin ? 'Remove admin' : 'Make admin'}
								</button>
								<button
									type="button"
									class={button}
									onclick={() => {
										resetId = resetId === u.id ? null : u.id;
										resetPassword = '';
									}}
								>
									Reset password
								</button>
								<button type="button" class={button} onclick={() => (removeId = u.id)}>
									Remove
								</button>
							{/if}
						</span>
					{/if}
				</div>
				{#if resetId === u.id}
					<form onsubmit={(e) => saveReset(e, u)} class="mt-2 flex items-end gap-2">
						<label class="min-w-0 flex-1">
							<span class={label}>New password for {u.username}</span>
							<input
								type="text"
								bind:value={resetPassword}
								required
								minlength="8"
								autocomplete="off"
								class={input}
							/>
						</label>
						<button type="submit" disabled={busy} class={primary}>Set</button>
					</form>
				{/if}
			</li>
		{/each}
	</ul>
{/if}
{#if notice}<p class="mt-2 text-sm text-emerald-700 dark:text-emerald-400" role="status">
		{notice}
	</p>{/if}
{#if error}<p class="mt-2 text-sm text-red-600" role="alert">{error}</p>{/if}
