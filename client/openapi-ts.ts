import { defineConfig } from '@hey-api/openapi-ts';

export default defineConfig({
	input: "http://localhost:8080/q/openapi?format=json",
	output: "src/client",
	plugins: [
		"@tanstack/svelte-query", 
 	],
});