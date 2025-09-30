import createClient from "openapi-fetch";
import type { paths } from "./schema.d.ts";

export const client = createClient<paths>({
	baseUrl: import.meta.env.VITE_API_URL || "http://localhost:8080/api",
	credentials: "include",
});
