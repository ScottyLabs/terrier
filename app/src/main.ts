import App from "./App.svelte";
import { mount } from "svelte";

const target = document.querySelector("#app");
if (!target) {
  throw new Error("missing #app mount target");
}

const app = mount(App, { target });

export default app;
