import { api } from "$lib/api/client";
import type { components } from "$lib/api/schema";

export type Session = {
  user: components["schemas"]["UserProfile"];
  csrfToken: string;
};

/** Returns the signed-in session, or `null` when nobody is signed in. */
export async function loadSession(): Promise<Session | null> {
  const { data, error } = await api.GET("/api/auth/status");
  if (error) {
    throw new Error(error.message);
  }
  if (!data.user || typeof data.csrf_token !== "string") {
    return null;
  }
  return { user: data.user, csrfToken: data.csrf_token };
}

/** Sign-in link that returns to the current page afterwards. */
export function loginUrl(): string {
  const here = `${location.pathname}${location.search}`;
  return `/api/auth/login?return_to=${encodeURIComponent(here)}`;
}
