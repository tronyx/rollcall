import type { User } from '#lib/api/types.ts';

/** The signed-in account, set by the root layout and the login page. */
export const session = $state<{ user: User | null }>({ user: null });
