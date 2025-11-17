import path from "node:path";
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig, loadEnv, type ViteDevServer } from "vite";
import oxlintPlugin from "vite-plugin-oxlint";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => {
	const env = loadEnv(mode, path.resolve(process.cwd(), ".."), "");

	return {
		plugins: [
			tailwindcss(),
			sveltekit(),
			oxlintPlugin(),
			{
				name: "supress-startup-urls",
				configureServer(server: ViteDevServer) {
					// Skip the startup messages because they have the wrong ports
					server.printUrls = () => {
						console.log(
							`  Visit \x1b[36m${env.APP_URL}\x1b[0m in your browser\n`,
						);
					};
				},
			},
		],

		// Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
		//
		// 1. prevent Vite from obscuring rust errors
		clearScreen: false,
		// 2. tauri expects a fixed port, fail if that port is not available
		server: {
			port: 1420,
			strictPort: true,
			host: host || false,
			allowedHosts: ["host.docker.internal"],
			hmr: host
				? {
						protocol: "ws",
						host,
						port: 1421,
					}
				: undefined,
			watch: {
				// 3. tell Vite to ignore watching `src-tauri`
				ignored: ["**/src-tauri/**"],
			},
		},
	};
});
