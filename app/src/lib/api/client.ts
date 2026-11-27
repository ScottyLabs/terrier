import createClient, { type Middleware } from "openapi-fetch";
import type { paths } from "./schema";

export const api = createClient<paths>({
  baseUrl: "/",
});

// The server rejects POST/PATCH/PUT without the session's CSRF token (see the authentication guide). Fetch it from /api/auth/status once,
// then attach it to every request that changes data.
let csrfToken: string | null = null;

async function getCsrfToken(): Promise<string | null> {
  if (csrfToken === null) {
    const { data } = await api.GET("/api/auth/status");
    csrfToken = data?.csrf_token ?? null;
  }
  return csrfToken;
}

const csrf: Middleware = {
  async onRequest({ request }) {
    if (request.method === "GET" || request.method === "HEAD") {
      return request;
    }
    const token = await getCsrfToken();
    if (token !== null) {
      request.headers.set("x-csrf-token", token);
    }
    return request;
  },
  onResponse({ response }) {
    // A 403 can mean the token is stale (e.g. after signing in again),
    // so fetch a fresh one next time.
    if (response.status === 403) {
      csrfToken = null;
    }
    return response;
  },
};

api.use(csrf);
