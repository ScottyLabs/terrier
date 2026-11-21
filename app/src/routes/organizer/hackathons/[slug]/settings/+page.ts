import { error } from "@sveltejs/kit";
import { api } from "$lib/api/client";
import type { PageLoad } from "./$types";

export const load: PageLoad = async ({ params }) => {
  const { data, response } = await api.GET("/api/hackathons/{slug}", {
    params: { path: { slug: params.slug } },
  });
  if (!data) error(response.status, "Hackathon not found");
  return { hackathon: data };
};
